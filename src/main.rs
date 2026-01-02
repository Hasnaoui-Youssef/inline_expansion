/*
 * How should the project structure look like?
 *We want to create an application, that given a c project, with make/cmake directives,
 *Generate a version of the project where every function call is expanded.
 *With a comment ahead specifying the function.
*/

use std::path::PathBuf;

use crate::{cli::Args, parser::{ast::AstParser, function_db::Signature}, call_graph::CallGraph};
use anyhow::Result;
use clap::Parser;


mod utils;
mod parser;
mod inliner;
mod rewriter;
mod cli;
mod call_graph;


fn main() -> Result<()> {
    let args = Args::parse();
    println!("Looking for compile_commands.json in {}", args.project_path.display());
    let build_path = args.project_path.join("compile_commands.json");
    if !build_path.exists() {
        anyhow::bail!(
            "Compile commands file not found at: {}",
            build_path.display()
        );
    }
    if !args.entry_point.exists() {
        anyhow::bail!(
            "Cannot find entry point : {}",
            args.entry_point.display()
        );
    }

    let parser = AstParser::new(&args.project_path)?;
    
    // Parse all files to get complete function database
    println!("\nParsing all source files...");
    let function_db = parser.parse_all_files()?;
    println!("Found {} functions in database", function_db.iter().count());

    // Build call graph from entry point (default to "main")
    let entry_func = "main";
    println!("\nBuilding call graph from '{}'...", entry_func);
    let call_graph = CallGraph::build(&function_db, entry_func)?;
    
    call_graph.print_summary();

    // Export graph visualizations
    let output_dir = args.project_path.join("call_graph_output");
    std::fs::create_dir_all(&output_dir)?;

    let dot_path = output_dir.join("call_graph.dot");
    call_graph.save_dot(&dot_path)?;
    println!("\nSaved DOT file to: {}", dot_path.display());

    let png_path = output_dir.join("call_graph.png");
    match call_graph.export_png(&png_path) {
        Ok(_) => println!("Saved PNG to: {}", png_path.display()),
        Err(e) => eprintln!("Warning: Could not generate PNG (is graphviz installed?): {}", e),
    }

    let svg_path = output_dir.join("call_graph.svg");
    match call_graph.export_svg(&svg_path) {
        Ok(_) => println!("Saved SVG to: {}", svg_path.display()),
        Err(e) => eprintln!("Warning: Could not generate SVG: {}", e),
    }

    // Print topological order
    println!("\nFunctions in call order (entry point first):");
    for (i, func_name) in call_graph.topological_order().iter().enumerate() {
        let node = call_graph.get_node(func_name).unwrap();
        let marker = if node.function.signature.return_type == "extern" {
            " (external)"
        } else if node.function.is_static {
            " (static)"
        } else {
            ""
        };
        println!("  {}. {}{}", i + 1, func_name, marker);
    }

    Ok(())
}

fn format_signature(sig : &Signature) -> String {
    let params = sig.args.iter()
        .map(|p| {
            if let Some(name) = &p.name {
                format!("{} {}", p.param_type, name)
            }else{
                p.param_type.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let variad = if sig.is_variadic { ",..." } else { "" };
    format!("{} {}({}{})", sig.return_type, sig.name, params, variad)
}
