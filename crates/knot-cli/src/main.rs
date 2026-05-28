use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "knot")]
#[command(about = "Multi-language static analysis engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze files and directories.
    Check {
        /// Files or directories to analyze.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { paths } => match knot_core::check_paths(&paths) {
            Ok(diagnostics) => {
                for diagnostic in diagnostics {
                    println!(
                        "{}[{}]: {}",
                        diagnostic.severity, diagnostic.rule_id, diagnostic.message
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
    }
}
