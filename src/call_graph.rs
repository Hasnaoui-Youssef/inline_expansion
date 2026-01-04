use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use graphviz_rust::cmd::{CommandArg, Format};
use graphviz_rust::printer::PrinterContext;
use graphviz_rust::dot_structures::*;
use graphviz_rust::dot_generator::*;

use crate::parser::function_db::{Definition, FunctionDatabase, CallInfo, CallContext};

#[derive(Debug, Clone)]
pub struct CallGraphNode {
    pub function: Arc<Definition>,
    pub calls: Vec<CallInfo>,
}

pub struct CallGraph {
    nodes: HashMap<String, CallGraphNode>,
    entry_point: String,

    // Graphviz elements to visualize our graph
    graph : graphviz_rust::dot_structures::Graph,
    printer_ctx : PrinterContext,
}

impl CallGraph {
    fn setup_graph() -> Graph{
        let mut graph = graph!(di id!("CallGraph"));
        graph.add_stmt(attr!("rankdir", "TB").into());
        graph.add_stmt(attr!("splines", "ortho").into());
        graph.add_stmt(attr!("nodesep", "0.8").into());
        graph.add_stmt(attr!("ranksep", "0.8").into());
        graph.add_stmt(attr!("fontname", "\"Helvetica\"").into());
        graph.add_stmt(GraphAttributes::new("node",vec![
                attr!("shape", "box"),
                attr!("fontname", "\"Helvetica\""),
                attr!("fontsize", "10")
        ]).into());
        graph.add_stmt(GraphAttributes::new("edge",vec![
                attr!("fontsize", "8")
        ]).into());

        graph
    }

    pub fn build(db: &FunctionDatabase, entry_point: &str) -> Result<Self> {
        let mut nodes = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(entry_point.to_string());

        while let Some(func_name) = queue.pop_front() {
            if visited.contains(&func_name) {
                continue;
            }
            visited.insert(func_name.clone());

            if let Some(def) = db.get_function_definition(&func_name) {
                // Queue callees for processing
                for call in &def.calls {
                    if !visited.contains(&call.function_name) {
                        queue.push_back(call.function_name.clone());
                    }
                }

                nodes.insert(func_name.clone(), CallGraphNode {
                    function: Arc::clone(&def),
                    calls: def.calls.clone(),
                });
            } else {
                // External function - no definition available
                nodes.insert(func_name.clone(), CallGraphNode {
                    function: Arc::new(Definition {
                        signature: crate::parser::function_db::Signature {
                            name: func_name.clone(),
                            return_type: "extern".to_string(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    calls: vec![],
                });
            }
        }

        let graph = Self::setup_graph();
        let mut printer_ctx = PrinterContext::default();

        printer_ctx
            .with_semi()
            .with_indent_step(4);


        Ok(CallGraph {
            nodes,
            entry_point: entry_point.to_string(),
            graph,
            printer_ctx
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.nodes.values().map(|n| n.calls.len()).sum()
    }

    pub fn to_dot(&mut self) {
        for (name, node) in &self.nodes {
            let node_id = Self::sanitize_id(name);
            let is_external = node.function.signature.return_type == "extern";
            let is_entry = name == &self.entry_point;

            let label = if is_external {
                format!("\"{}\\n(external)\"", name)
            } else {
                let source = node.function.source_file
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("?");
                format!("\"{}\\n{}\"", name, source)
            };

            let (fillcolor, style) = if is_entry {
                ("\"#90EE90\"", "filled")
            } else if is_external {
                ("\"#D3D3D3\"", "\"filled,dashed\"")
            } else if node.function.is_static {
                ("\"#FFFACD\"", "filled")
            } else {
                ("\"#E6F3FF\"", "filled")
            };

            self.graph.add_stmt(
                node!(node_id.to_string();
                    attr!("label", label.to_string()),
                    attr!("fillcolor", fillcolor),
                    attr!("style", style))
                .into());
        }


        // Add edges with order labels and context-based styling
        for (name, node) in &self.nodes {
            let from_id = Self::sanitize_id(name);

            for call in &node.calls {
                let to_id = Self::sanitize_id(&call.function_name);

                match &call.context {
                    CallContext::Sequential => {
                        let label = format!("\"{}\"",call.order);
                        self.graph.add_stmt(edge!(node_id!(from_id) => node_id!(to_id),
                        vec![
                            attr!("color", "\"#333333\""),
                            attr!("label", label.to_string()),
                        ]).into());
                    }
                    CallContext::Conditional { branch_id } => {
                        let label = format!("\"{}:if{}\"",call.order, branch_id);
                        self.graph.add_stmt(edge!(node_id!(from_id) => node_id!(to_id),
                        vec![
                            attr!("color", "\"#333333\""),
                            attr!("style", "dashed"),
                            attr!("label", label.to_string()),
                        ]).into());
                    }
                    CallContext::Loop => {
                        let label = format!("\"{}:loop\"",call.order);
                        self.graph.add_stmt(edge!(node_id!(from_id) => node_id!(to_id),
                        vec![
                            attr!("color", "\"#4ECDC4\""),
                            attr!("style", "bold"),
                            attr!("label", label.to_string()),
                        ]).into());
                    }
                    CallContext::Switch { case_id } => {
                        let label = format!("\"{}:case{}\"",call.order, case_id);
                        self.graph.add_stmt(edge!(node_id!(from_id) => node_id!(to_id),
                        vec![
                            attr!("color", "\"#9B59B6\""),
                            attr!("label", label.to_string()),
                        ]).into());
                    }
                };

            }
        }
    }

    /// Export the graph to a PNG file
    pub fn export_png(&mut self, output_path: &Path) -> Result<()> {
        graphviz_rust::exec(
            &self.graph,
            &mut self.printer_ctx,
            vec![
                CommandArg::Format(Format::Png),
                CommandArg::Output(output_path.to_string_lossy().to_string()),
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to generate PNG: {}", e))?;

        Ok(())
    }

    pub fn export_svg(&mut self, output_path: &Path) -> Result<()> {

        graphviz_rust::exec(
            &self.graph,
            &mut self.printer_ctx,
            vec![
                CommandArg::Format(Format::Svg),
                CommandArg::Output(output_path.to_string_lossy().to_string()),
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to generate SVG: {}", e))?;

        Ok(())
    }

    /// Save the DOT file
    pub fn save_dot(&mut self, output_path: &Path) -> Result<()> {
        std::fs::write(
            output_path,
            graphviz_rust::print(
                &self.graph,
                &mut self.printer_ctx
            ))?;
        Ok(())
    }

    fn sanitize_id(name: &str) -> String {
        name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
    }

    /// Print a summary of the call graph
    pub fn print_summary(&self) {
        println!("Call Graph Summary:");
        println!("  Entry point: {}", self.entry_point);
        println!("  Total nodes: {}", self.node_count());
        println!("  Total edges: {}", self.edge_count());

        let external_count = self.nodes.values()
            .filter(|n| n.function.signature.return_type == "extern")
            .count();
        let static_count = self.nodes.values()
            .filter(|n| n.function.is_static)
            .count();

        println!("  External functions: {}", external_count);
        println!("  Static functions: {}", static_count);
    }
}
