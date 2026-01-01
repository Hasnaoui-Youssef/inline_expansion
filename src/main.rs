/*
 * How should the project structure look like?
 *We want to create an application, that given a c project, with make/cmake directives,
 *Generate a version of the project where every function call is expanded.
 *With a comment ahead specifying the function.
*/

use crate::cli::Args;
use anyhow::{Context, Result};
use clap::Parser;


mod utils;
mod parser;
mod inliner;
mod rewriter;
mod cli;


fn main() -> Result<()> {
    let args = Args::parse();
    println!("Looking for compile_commands.json in {}", args.project_path.display());
    let file_path = args.project_path.join("compile_commands.json");
    if !file_path.exists() {
        anyhow::bail!(
            "Compile commands file not found at: {}",
            file_path.display()
        );
    }
    println!("Found file {:?}", file_path);
    Ok(())
}
