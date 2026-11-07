mod strategy;

use clap::{ArgMatches, Command};

#[derive(Debug, PartialEq, Eq)]
pub enum EngineCommand {
    Strategy(strategy::StrategyCommand),
}

impl EngineCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Strategy(cmd) => cmd.execute(),
        }
    }
}

pub fn build_cli() -> Command {
    Command::new("bte")
        .about("Backtesting Engine (Rust CLI)")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(strategy::build_cli())
}

pub fn parse(matches: &ArgMatches) -> anyhow::Result<EngineCommand> {
    match matches.subcommand() {
        Some((strategy::STRATEGY, m)) => Ok(EngineCommand::Strategy(strategy::parse(m)?)),
        Some((name, _)) => anyhow::bail!("unhandled engine subcommand '{name}'"),
        None => anyhow::bail!("missing engine subcommand"),
    }
}
