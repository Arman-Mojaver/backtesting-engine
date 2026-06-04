use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bt-engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Health check — returns "pong"
    Ping,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Ping => println!("pong"),
    }
    Ok(())
}
