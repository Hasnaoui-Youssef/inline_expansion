use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::fmt::Write;

use anyhow::Result;
use graphviz_rust::cmd::{CommandArg, Format};
use graphviz_rust::printer::PrinterContext;

use crate::parser::function_db::{Definition, FunctionDatabase, CallInfo, CallContext};

#[derive(Debug, Clone)]
pub struct CallGraphNode {
    pub function: Arc<Definition>,
    /// Ordered list of calls with context information
    pub calls: Vec<CallInfo>,
}

#[derive(Debug)]
pub struct CallGraph {
    nodes: HashMap<String, CallGraphNode>,
    entry_point: String,
}

impl CallGraph {
    /// Build a call graph starting from the entry point function.
    /// Only includes functions reachable from the entry point.
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

        Ok(CallGraph {
            nodes,
            entry_point: entry_point.to_string(),
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.nodes.values().map(|n| n.calls.len()).sum()
    }

    pub fn get_node(&self, name: &str) -> Option<&CallGraphNode> {
        self.nodes.get(name)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (&String, &CallGraphNode)> {
        self.nodes.iter()
    }

    /// Generate DOT representation with sequential tree layout
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        
        writeln!(dot, "digraph CallGraph {{").unwrap();
        // Use dot layout with orthogonal edges (splines=ortho avoids crossing through nodes)
        writeln!(dot, "    rankdir=TB;").unwrap();
        writeln!(dot, "    splines=ortho;").unwrap();
        writeln!(dot, "    nodesep=0.5;").unwrap();
        writeln!(dot, "    ranksep=0.8;").unwrap();
        writeln!(dot, "    fontname=\"Helvetica\";").unwrap();
        writeln!(dot, "    node [shape=box, fontname=\"Helvetica\", fontsize=10];").unwrap();
        writeln!(dot, "    edge [fontsize=8];").unwrap();
        writeln!(dot).unwrap();

        // Add all function nodes
        for (name, node) in &self.nodes {
            let node_id = Self::sanitize_id(name);
            let is_external = node.function.signature.return_type == "extern";
            let is_entry = name == &self.entry_point;

            let label = if is_external {
                format!("{}\\n(external)", name)
            } else {
                let source = node.function.source_file
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("?");
                format!("{}\\n{}", name, source)
            };

            let style = if is_entry {
                "fillcolor=\"#90EE90\", style=filled"
            } else if is_external {
                "fillcolor=\"#D3D3D3\", style=\"filled,dashed\""
            } else if node.function.is_static {
                "fillcolor=\"#FFFACD\", style=filled"
            } else {
                "fillcolor=\"#E6F3FF\", style=filled"
            };

            writeln!(dot, "    {} [label=\"{}\", {}];", node_id, label, style).unwrap();
        }

        writeln!(dot).unwrap();

        // Add edges with order labels and context-based styling
        for (name, node) in &self.nodes {
            let from_id = Self::sanitize_id(name);
            
            for call in &node.calls {
                let to_id = Self::sanitize_id(&call.function_name);
                
                let (edge_style, edge_label) = match &call.context {
                    CallContext::Sequential => {
                        ("color=\"#333333\"".to_string(), format!("{}", call.order))
                    }
                    CallContext::Conditional { branch_id } => {
                        (format!("color=\"#FF6B6B\", style=dashed"), 
                         format!("{}:if{}", call.order, branch_id))
                    }
                    CallContext::Loop => {
                        ("color=\"#4ECDC4\", style=bold".to_string(),
                         format!("{}:loop", call.order))
                    }
                    CallContext::Switch { case_id } => {
                        (format!("color=\"#9B59B6\""),
                         format!("{}:case{}", call.order, case_id))
                    }
                };

                writeln!(dot, "    {} -> {} [label=\"{}\", {}];", 
                         from_id, to_id, edge_label, edge_style).unwrap();
            }
        }

        writeln!(dot, "}}").unwrap();
        dot
    }

    /// Generate a hierarchical tree DOT focusing on a single function's call sequence
    pub fn to_dot_for_function(&self, func_name: &str) -> Option<String> {
        let node = self.nodes.get(func_name)?;
        let mut dot = String::new();
        
        writeln!(dot, "digraph {} {{", Self::sanitize_id(func_name)).unwrap();
        writeln!(dot, "    rankdir=TB;").unwrap();
        writeln!(dot, "    splines=ortho;").unwrap();
        writeln!(dot, "    node [shape=box, fontname=\"Helvetica\"];").unwrap();
        writeln!(dot).unwrap();

        // Group calls by context for subgraph clustering
        let mut sequential_calls = Vec::new();
        let mut conditional_groups: HashMap<u32, Vec<&CallInfo>> = HashMap::new();
        let mut loop_calls = Vec::new();
        let mut switch_groups: HashMap<u32, Vec<&CallInfo>> = HashMap::new();

        for call in &node.calls {
            match &call.context {
                CallContext::Sequential => sequential_calls.push(call),
                CallContext::Conditional { branch_id } => {
                    conditional_groups.entry(*branch_id).or_default().push(call);
                }
                CallContext::Loop => loop_calls.push(call),
                CallContext::Switch { case_id } => {
                    switch_groups.entry(*case_id).or_default().push(call);
                }
            }
        }

        // Entry node
        writeln!(dot, "    {} [label=\"{}\", fillcolor=\"#90EE90\", style=filled];", 
                 Self::sanitize_id(func_name), func_name).unwrap();

        // Add conditional subgraphs
        for (branch_id, calls) in &conditional_groups {
            writeln!(dot, "    subgraph cluster_if{} {{", branch_id).unwrap();
            writeln!(dot, "        label=\"Branch {}\";", branch_id).unwrap();
            writeln!(dot, "        style=dashed;").unwrap();
            writeln!(dot, "        color=\"#FF6B6B\";").unwrap();
            for call in calls {
                let call_node_id = format!("{}_{}", Self::sanitize_id(&call.function_name), call.order);
                writeln!(dot, "        {} [label=\"{}\"];", call_node_id, call.function_name).unwrap();
            }
            writeln!(dot, "    }}").unwrap();
        }

        // Add loop subgraph
        if !loop_calls.is_empty() {
            writeln!(dot, "    subgraph cluster_loop {{").unwrap();
            writeln!(dot, "        label=\"Loop\";").unwrap();
            writeln!(dot, "        style=bold;").unwrap();
            writeln!(dot, "        color=\"#4ECDC4\";").unwrap();
            for call in &loop_calls {
                let call_node_id = format!("{}_{}", Self::sanitize_id(&call.function_name), call.order);
                writeln!(dot, "        {} [label=\"{}\"];", call_node_id, call.function_name).unwrap();
            }
            writeln!(dot, "    }}").unwrap();
        }

        // Add sequential calls
        for call in &sequential_calls {
            let call_node_id = format!("{}_{}", Self::sanitize_id(&call.function_name), call.order);
            writeln!(dot, "    {} [label=\"{}\"];", call_node_id, call.function_name).unwrap();
        }

        writeln!(dot).unwrap();

        // Add edges in order
        let mut sorted_calls: Vec<_> = node.calls.iter().collect();
        sorted_calls.sort_by_key(|c| c.order);

        let mut prev_node = Self::sanitize_id(func_name);
        for call in sorted_calls {
            let call_node_id = format!("{}_{}", Self::sanitize_id(&call.function_name), call.order);
            writeln!(dot, "    {} -> {} [label=\"{}\"];", prev_node, call_node_id, call.order).unwrap();
            prev_node = call_node_id;
        }

        writeln!(dot, "}}").unwrap();
        Some(dot)
    }

    /// Export the graph to a PNG file
    pub fn export_png(&self, output_path: &Path) -> Result<()> {
        let dot = self.to_dot();
        
        graphviz_rust::exec(
            graphviz_rust::parse(&dot).map_err(|e| anyhow::anyhow!("Failed to parse DOT: {}", e))?,
            &mut PrinterContext::default(),
            vec![
                CommandArg::Format(Format::Png),
                CommandArg::Output(output_path.to_string_lossy().to_string()),
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to generate PNG: {}", e))?;

        Ok(())
    }

    /// Export the graph to an SVG file
    pub fn export_svg(&self, output_path: &Path) -> Result<()> {
        let dot = self.to_dot();
        
        graphviz_rust::exec(
            graphviz_rust::parse(&dot).map_err(|e| anyhow::anyhow!("Failed to parse DOT: {}", e))?,
            &mut PrinterContext::default(),
            vec![
                CommandArg::Format(Format::Svg),
                CommandArg::Output(output_path.to_string_lossy().to_string()),
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to generate SVG: {}", e))?;

        Ok(())
    }

    /// Save the DOT file
    pub fn save_dot(&self, output_path: &Path) -> Result<()> {
        std::fs::write(output_path, self.to_dot())?;
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

    /// Get functions in topological order (callers before callees where possible)
    pub fn topological_order(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        fn visit(
            name: &str,
            nodes: &HashMap<String, CallGraphNode>,
            visited: &mut HashSet<String>,
            temp_visited: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            if temp_visited.contains(name) {
                // Cycle detected, skip
                return;
            }
            temp_visited.insert(name.to_string());

            if let Some(node) = nodes.get(name) {
                for call in &node.calls {
                    visit(&call.function_name, nodes, visited, temp_visited, result);
                }
            }

            temp_visited.remove(name);
            visited.insert(name.to_string());
            result.push(name.to_string());
        }

        visit(&self.entry_point, &self.nodes, &mut visited, &mut temp_visited, &mut result);

        // Reverse to get callers before callees
        result.reverse();
        result
    }
}
