use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod json;

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
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Files or directories to analyze.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
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
        Command::Check { format, paths } => {
            let diagnostics = knot_core::check_paths(&paths)?;

            match format {
                OutputFormat::Human => {
                    for diagnostic in diagnostics {
                        println!("{}", render_diagnostic(&diagnostic));
                    }
                }
                OutputFormat::Json => {
                    println!("{}", json::render_diagnostics(&diagnostics)?);
                }
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
