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
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { paths } => {
            let diagnostics = knot_core::check_paths(&paths)?;

            for diagnostic in diagnostics {
                println!("{}", render_diagnostic(&diagnostic));
            }

            Ok(())
        }
    }
}

fn render_diagnostic(diagnostic: &knot_core::Diagnostic) -> String {
    let body = format!(
        "{}[{}]: {}",
        diagnostic.severity, diagnostic.rule_id, diagnostic.message
    );

    match &diagnostic.span {
        Some(span) => format!(
            "{}:{}:{}: {body}",
            span.file, span.start.line, span.start.column
        ),
        None => body,
    }
}
