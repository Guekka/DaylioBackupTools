use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use daylio_tools::tools::{ToolCommands, process_command};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: RootCommands,
}

#[derive(Subcommand)]
enum RootCommands {
    /// Tools for working with Daylio files
    #[command(subcommand)]
    Tool(ToolCommands),
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        RootCommands::Tool(tool_commands) => process_command(tool_commands)?,
    }

    Ok(())
}
