mod strategy;

use clap::{ArgMatches, Command};

pub enum EngineCommand {
    Strategy(strategy::StrategyCommand),
}

pub fn build_cli() -> Command {
    Command::new("bt-engine")
        .about("Backtesting Engine (Rust CLI)")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(strategy::build_strategy_cli())
}

pub fn parse(matches: &ArgMatches) -> anyhow::Result<EngineCommand> {
    match matches.subcommand() {
        Some(("strategy", m)) => Ok(EngineCommand::Strategy(strategy::parse_strategy(m)?)),
        _ => anyhow::bail!("invalid engine command"),
    }
}

impl EngineCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            EngineCommand::Strategy(cmd) => cmd.execute(),
        }
    }
}
