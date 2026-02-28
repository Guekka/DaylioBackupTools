use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use daylio_tools::server::serve;
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
    Serve {
        /// Host to serve on
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to serve on
        #[arg(short, long, default_value = "14279")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        RootCommands::Tool(tool_commands) => process_command(tool_commands)?,
        RootCommands::Serve { host, port } => {
            println!("Serving on {host}:{port}...");
            serve(host, port).await?;
        }
    }

    Ok(())
}
