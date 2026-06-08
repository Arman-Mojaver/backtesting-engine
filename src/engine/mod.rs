mod redis;
mod strategy;

use clap::{ArgMatches, Command};

pub enum EngineCommand {
    Redis(redis::RedisCommand),
    Strategy(strategy::StrategyCommand),
}

pub fn build_cli() -> Command {
    Command::new("bt-engine")
        .about("Backtesting Engine (Rust CLI)")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(redis::build_redis_cli())
        .subcommand(strategy::build_strategy_cli())
}

pub fn parse(matches: ArgMatches) -> EngineCommand {
    match matches.subcommand() {
        Some(("redis", m)) => EngineCommand::Redis(redis::parse_redis(m)),
        Some(("strategy", m)) => EngineCommand::Strategy(strategy::parse_strategy(m)),
        _ => unreachable!("invalid engine command"),
    }
}

impl EngineCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            EngineCommand::Redis(cmd) => cmd.execute(),
            EngineCommand::Strategy(cmd) => cmd.execute(),
        }
    }
}
