use std::path::{Path, PathBuf};

use clang::{Clang, CompilationDatabase, Entity, EntityKind, Index, StorageClass};
use anyhow::Result;


use super::function_db::{FunctionDatabase, Definition, Signature, Parameter, CallInfo, CallContext};

/// Tracks the current context while traversing the AST
#[derive(Debug, Clone, Default)]
struct CallCollector {
    calls: Vec<CallInfo>,
    order_counter: u32,
    context_stack: Vec<CallContext>,
    branch_counter: u32,
    case_counter: u32,
}

impl CallCollector {
    fn new() -> Self {
        Self::default()
    }

    fn current_context(&self) -> CallContext {
        self.context_stack.last().cloned().unwrap_or(CallContext::Sequential)
    }

    fn depth(&self) -> u32 {
        self.context_stack.len() as u32
    }

    fn push_conditional(&mut self) {
        self.branch_counter += 1;
        self.context_stack.push(CallContext::Conditional { branch_id: self.branch_counter });
    }

    fn push_loop(&mut self) {
        self.context_stack.push(CallContext::Loop);
    }

    fn push_switch_case(&mut self) {
        self.case_counter += 1;
        self.context_stack.push(CallContext::Switch { case_id: self.case_counter });
    }

    fn pop_context(&mut self) {
        self.context_stack.pop();
    }

    fn add_call(&mut self, function_name: String, line: u32, column: u32) {
        self.order_counter += 1;
        self.calls.push(CallInfo {
            function_name,
            line,
            column,
            order: self.order_counter,
            context: self.current_context(),
            context_depth: self.depth(),
        });
    }
}

pub struct AstParser{
    clang : Clang,
    compilation_db : CompilationDatabase,
    project_root : PathBuf,
}

impl AstParser {
    pub fn new(build_path : &Path) -> Result<Self> {
        let clang = Clang::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize Clang parser : {}", e))?;
        let project_root = build_path.canonicalize()
            .map_err(|e| anyhow::anyhow!("Failed to canonicalize project path: {}", e))?;
        let db = CompilationDatabase::from_directory(&project_root)
            .map_err(|_| anyhow::anyhow!(format!( "Failed to load compile_commands.json from {}",
                    project_root.display()
            )))?;

        Ok(AstParser {clang, compilation_db : db, project_root})
    }
    pub fn parse_file(&self, file_path : &Path) -> Result<FunctionDatabase> {
        // Make file path absolute before changing directory
        let abs_file_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(file_path).canonicalize()
                .map_err(|e| anyhow::anyhow!("Failed to resolve file path {}: {}", file_path.display(), e))?
        };

        // Change to project directory so relative include paths work
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&self.project_root)?;

        let result = self.parse_file_impl(&abs_file_path);

        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        result
    }

    fn parse_file_impl(&self, file_path : &Path) -> Result<FunctionDatabase> {
        let comp_commands = self.compilation_db.get_compile_commands(file_path)
            .map_err(|_| anyhow::anyhow!("Failed to get compile commands"))?;
        let commands =  comp_commands.get_commands();
        if commands.is_empty() {
            anyhow::bail!(
                "No compilation commands found for {} in the compilation database",
                file_path.display()
            );
        }
        let command = &commands[0];
        let mut args = Self::extract_clang_compatible_flags(&command.get_arguments()[1..]);
        args.push("-ferror-limit=0".to_string());
        args.push("-Wno-everything".to_string());

        let index = Index::new(&self.clang, true, true);

        let tu_result = index.parser(file_path)
            .arguments(&args)
            .skip_function_bodies(false)
            .detailed_preprocessing_record(true)
            .parse();

        let tu = match tu_result {
            Ok(tu) => {
                tu
            }
            Err(e) => {
                eprintln!("Parser failed with filtered args: {}", e);
                eprintln!("Args used: {:?}", args);
                let minimal_tu = index
                    .parser(file_path)
                    .arguments(&vec!["-ICore/Inc"])
                    .parse()
                    .map_err(|err| anyhow::anyhow!("Failed to parse {} : {}", file_path.display(), err))?;

                minimal_tu
            }
        };

        let mut function_db = FunctionDatabase::new();

        self.collect_functions(&tu.get_entity(), &mut function_db)?;

        Ok(function_db)
    }

    /// Parse all source files in the compilation database to build a complete function database
    pub fn parse_all_files(&self) -> Result<FunctionDatabase> {
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&self.project_root)?;

        let result = self.parse_all_files_impl();

        std::env::set_current_dir(original_dir)?;
        result
    }

    fn parse_all_files_impl(&self) -> Result<FunctionDatabase> {
        let mut function_db = FunctionDatabase::new();
        let index = Index::new(&self.clang, true, true);

        let all_commands = self.compilation_db.get_all_compile_commands();
        for command in all_commands.get_commands() {
            let file_path = command.get_filename();
            
            // Skip non-C files (like assembly)
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "c" && ext != "h" {
                continue;
            }

            let mut args = Self::extract_clang_compatible_flags(&command.get_arguments()[1..]);
            args.push("-ferror-limit=0".to_string());
            args.push("-Wno-everything".to_string());

            let tu_result = index.parser(&file_path)
                .arguments(&args)
                .skip_function_bodies(false)
                .detailed_preprocessing_record(true)
                .parse();

            if let Ok(tu) = tu_result {
                let _ = self.collect_functions(&tu.get_entity(), &mut function_db);
            } else {
                eprintln!("Warning: Failed to parse {}", file_path.display());
            }
        }

        Ok(function_db)
    }

    fn collect_functions(&self, entity : &Entity, db : &mut FunctionDatabase) -> Result<()>{
        if let Some(location) = entity.get_location() {
            if location.is_in_system_header() {
                return Ok(());
            }
        }
        if entity.get_kind() == EntityKind::FunctionDecl {
            if entity.is_definition() {
                if let Some(def) = self.extract_function_definition(entity)? {
                    db.add_function_ref(&def);
                }
            }
        }
        for child in entity.get_children() {
            self.collect_functions(&child, db)?;
        }
        Ok(())
    }

    pub fn extract_function_definition(&self, entity : &Entity) -> Result<Option<Definition>> {
        if entity.get_kind() != EntityKind::FunctionDecl || !entity.is_definition() {
            ()
        }
        let name = match entity.get_name() {
            Some(n) => n,
            None => return Ok(None)
        };
        let return_type = entity.get_result_type().map(|t| t.get_display_name()).unwrap_or_else(|| "void".to_string());
        let params = entity.get_arguments().unwrap_or_default();
        let args = params
            .iter()
            .map(|arg| {
                let name = arg.get_name();
                let param_type = arg.get_type()
                    .map(|t| t.get_display_name())
                    .unwrap_or_else(|| "unknown".to_string());
                Parameter {
                    name,
                    param_type
                }
            })
            .collect();
        let is_variadic = entity.is_variadic();

        let signature = Signature {
            name,
            return_type,
            args,
            is_variadic
        };

        let source_file = entity.get_location()
            .and_then(|loc|{
                loc.get_file_location().file.map(|f| f.get_path())
            })
            .unwrap_or_else( || std::path::PathBuf::from("<unknown>"));

        let body = self.extract_function_body(entity)?;
        let is_static = entity.get_storage_class() == Some(StorageClass::Static);
        let calls = self.collect_calls(entity);

        Ok(Some(Definition {
            signature,
            body,
            source_file,
            is_static,
            calls,
        }))

    }

    fn collect_calls(&self, entity: &Entity) -> Vec<CallInfo> {
        let mut collector = CallCollector::new();
        self.collect_calls_recursive(entity, &mut collector);
        collector.calls
    }

    fn collect_calls_recursive(&self, entity: &Entity, collector: &mut CallCollector) {
        let kind = entity.get_kind();

        // Handle different control flow constructs
        match kind {
            EntityKind::IfStmt => {
                let children: Vec<_> = entity.get_children();
                // IfStmt has: condition, then-branch, [else-branch]
                if let Some(condition) = children.get(0) {
                    self.collect_calls_recursive(condition, collector);
                }
                if let Some(then_branch) = children.get(1) {
                    collector.push_conditional();
                    self.collect_calls_recursive(then_branch, collector);
                    collector.pop_context();
                }
                if let Some(else_branch) = children.get(2) {
                    collector.push_conditional();
                    self.collect_calls_recursive(else_branch, collector);
                    collector.pop_context();
                }
                return;
            }
            EntityKind::WhileStmt | EntityKind::ForStmt | EntityKind::DoStmt => {
                collector.push_loop();
                for child in entity.get_children() {
                    self.collect_calls_recursive(&child, collector);
                }
                collector.pop_context();
                return;
            }
            EntityKind::SwitchStmt => {
                let children: Vec<_> = entity.get_children();
                // First child is the condition
                if let Some(condition) = children.get(0) {
                    self.collect_calls_recursive(condition, collector);
                }
                // Rest are case statements
                for child in children.iter().skip(1) {
                    self.collect_calls_recursive(child, collector);
                }
                return;
            }
            EntityKind::CaseStmt | EntityKind::DefaultStmt => {
                collector.push_switch_case();
                for child in entity.get_children() {
                    self.collect_calls_recursive(&child, collector);
                }
                collector.pop_context();
                return;
            }
            EntityKind::CallExpr => {
                if let Some(referenced) = entity.get_reference() {
                    if let Some(name) = referenced.get_name() {
                        let (line, column) = entity.get_location()
                            .map(|loc| {
                                let file_loc = loc.get_file_location();
                                (file_loc.line, file_loc.column)
                            })
                            .unwrap_or((0, 0));
                        collector.add_call(name, line, column);
                    }
                }
            }
            _ => {}
        }

        for child in entity.get_children() {
            self.collect_calls_recursive(&child, collector);
        }
    }

    pub fn extract_function_body(&self, entity : &Entity) -> Result<String> {
        for child in entity.get_children() {
            if child.get_kind() == EntityKind::CompoundStmt {
                if let Some(body_range) = child.get_range() {
                    let body = body_range.tokenize()
                        .iter()
                        .map(|token| {
                            token.get_spelling()
                        })
                    .collect::<Vec<String>>()
                        .join(" ");
                    return Ok(body);
                }
            }
        }
        Ok(String::new())
    }

    /// Extract only -D (defines) and -I (includes) flags, which are the only ones
    /// that affect AST parsing. This avoids GCC/ARM-specific flag incompatibilities.
    fn extract_clang_compatible_flags(args: &[String]) -> Vec<String> {
        let mut filtered = Vec::new();

        for arg in args {
            if arg.starts_with("-D") || arg.starts_with("-I") {
                filtered.push(arg.clone());
            }
        }

        filtered
    }

}
