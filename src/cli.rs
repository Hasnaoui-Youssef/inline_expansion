use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "code-inliner")]
#[command(about = "Inline function calls in main")]
pub struct Args {
    #[arg(short, long, value_name="DIR")]
    pub project_path : PathBuf,

    #[arg(short, long, value_name="ENTRY_POINT")]
    pub entry_point : PathBuf
}


