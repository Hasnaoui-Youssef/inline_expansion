use crate::{cli::Args, parser::{ast::AstParser}, call_graph::CallGraph};
use anyhow::Result;
use clap::Parser;


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
    if !args.entry_file.exists() {
        anyhow::bail!(
            "Cannot find entry point : {}",
            args.entry_file.display()
        );
    }

    let parser = AstParser::new(&args.project_path)?;

    println!("\nParsing all source files...");
    let function_db = parser.parse_all_files(false)?;
    println!("Found {} functions in database", function_db.iter().count());

    let entry_func = "main";
    let mut call_graph = CallGraph::build(&function_db, entry_func)?;

    call_graph.to_dot();

    call_graph.print_summary();

    let original_dir = std::env::current_dir()?;
    let output_dir = original_dir.join("call_graph_output");
    std::fs::create_dir_all(&output_dir)?;

    let dot_path = output_dir.join("call_graph.dot");
    call_graph.save_dot(&dot_path)?;
    println!("\nSaved DOT file to: {}", dot_path.display());

    let png_path = output_dir.join("call_graph.png");
    match call_graph.export_png(&png_path) {
        Ok(_) => println!("Saved PNG to: {}", png_path.display()),
        Err(e) => eprintln!("Warning: Could not generate PNG: {}", e),
    }

    let svg_path = output_dir.join("call_graph.svg");
    match call_graph.export_svg(&svg_path) {
        Ok(_) => println!("Saved SVG to: {}", svg_path.display()),
        Err(e) => eprintln!("Warning: Could not generate SVG: {}", e),
    }

    Ok(())
}
