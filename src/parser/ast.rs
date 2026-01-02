use std::path::Path;

use clang::{Clang, CompilationDatabase, Entity, EntityKind, Type, StorageClass};
use anyhow::{Context, Result};


use super::function_db::{FunctionDatabase, Definition, Signature, Parameter};

pub struct AstParser {
    clang : Clang,
    compilation_db : CompilationDatabase,
    entry_candidates : Vec<Path>,
}

impl AstParser {
    pub fn new(build_path : &Path) -> Result<Self> {
        let clang = Clang::new().context("Failed to initialize Clang parser")?;
        let db = CompilationDatabase::from_directory(build_path)
            .context(format!(
                    "Failed to load compile_commands.json from {}",
                    build_path.display()
            ))?;

        Ok(AstParser {clang,entry_candidates: Vec::new(), compilation_db : db})
    }
    pub fn parse_file(&self, file_path : &Path) -> Result<FunctionDatabase> {
        let commands = self.compilation_db.get_compile_commands(file_path)?;
        if commands.get_commands().is_empty() {
            anyhow::bail!(
                "No compilation commands found for {} in the compilation database",
                file_path.display()
            );
        }
    }

    pub fn extract_function_definition(&self, entity : &Entity) -> Result<Option<Definition>> {
        if !entity.get_kind() == EntityKind::FunctionDecl || !entity.is_definition() {
            Ok(None)
        }
        let name = match entity.get_name() {
            Some(n) => n,
            None => return Ok(None)
        };
        let return_type = entity.get_result_type().unwrap_or_else(|| "void".to_string());
        let params = entity.get_arguments().unwrap_or_default();
        let args = params
            .iter()
            .map(|arg| {
                let name = arg.get_name();
                let param_type = arg.get_type()
                    .map(|t| t.get_display_name())
                    .unwrap_or_else("unknown".to_string());
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
                loc.get_file_location.file.map(|f| f.get_path())
            })
            .unwrap_or_else( || std::path::PathBuf::from("<unknown>"));

        let body = self.extract_function_body(entity);
        let is_static = entity.get_storage_class() == StorageClass::Static;

        Ok(Some(Defintion {
            signature,
            body,
            source_file,
            is_static,
        }))

    }

    pub fn extract_function_body(&self, entity : &Entity) -> Result<String> {
        if let Some(range) = entity.get_range() {
            for child in entity.get_children() {
                if child.get_kind == EntityKind::CompoundStmt {
                    if let Some(body_range) = child.get_range() {
                        let tu = entity.get_translation_unit()
                            .context("No translation unit found")?;
                        let body = body_range.tokenize()
                            .iter()
                            .map(|token| {
                                token.get_spelling()
                            })
                            .collect()
                            .join(" ");
                        Ok(body)
                    }
                }
            }
        }
        Ok(String::new())
    }
}
