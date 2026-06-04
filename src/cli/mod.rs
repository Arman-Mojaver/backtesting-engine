mod dev;
mod indicator;
mod money_management_strategy;
mod operation_point;
mod raw_point;
mod redis;
mod resampled_point;
mod strategy;

use clap::{ArgMatches, Command};
use std::process::Command as ProcessCommand;

pub enum AppCommand {
    Dev(dev::DevCommand),
    Indicator(indicator::IndicatorCommand),
    MoneyManagementStrategy(money_management_strategy::MoneyManagementStrategyCommand),
    OperationPoint(operation_point::OperationPointCommand),
    RawPoint(raw_point::RawPointCommand),
    Redis(redis::RedisCommand),
    ResampledPoint(resampled_point::ResampledPointCommand),
    Strategy(strategy::StrategyCommand),
}

pub fn build_cli() -> Command {
    Command::new("bt")
        .about("Backtesting Engine CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(dev::build_dev_cli())
        .subcommand(indicator::build_indicator_cli())
        .subcommand(money_management_strategy::build_money_management_strategy_cli())
        .subcommand(operation_point::build_operation_point_cli())
        .subcommand(raw_point::build_raw_point_cli())
        .subcommand(redis::build_redis_cli())
        .subcommand(resampled_point::build_resampled_point_cli())
        .subcommand(strategy::build_strategy_cli())
}

pub fn parse(matches: ArgMatches) -> AppCommand {
    match matches.subcommand() {
        Some(("dev", m)) => AppCommand::Dev(dev::parse_dev(m)),
        Some(("indicator", m)) => AppCommand::Indicator(indicator::parse_indicator(m)),
        Some(("money-management-strategy", m)) => AppCommand::MoneyManagementStrategy(
            money_management_strategy::parse_money_management_strategy(m),
        ),
        Some(("operation-point", m)) => AppCommand::OperationPoint(operation_point::parse_operation_point(m)),
        Some(("raw-point", m)) => AppCommand::RawPoint(raw_point::parse_raw_point(m)),
        Some(("redis", m)) => AppCommand::Redis(redis::parse_redis(m)),
        Some(("resampled-point", m)) => AppCommand::ResampledPoint(resampled_point::parse_resampled_point(m)),
        Some(("strategy", m)) => AppCommand::Strategy(strategy::parse_strategy(m)),
        _ => unreachable!("invalid top-level command"),
    }
}

impl AppCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            AppCommand::Dev(cmd) => cmd.execute(),
            AppCommand::Indicator(cmd) => cmd.execute(),
            AppCommand::MoneyManagementStrategy(cmd) => cmd.execute(),
            AppCommand::OperationPoint(cmd) => cmd.execute(),
            AppCommand::RawPoint(cmd) => cmd.execute(),
            AppCommand::Redis(cmd) => cmd.execute(),
            AppCommand::ResampledPoint(cmd) => cmd.execute(),
            AppCommand::Strategy(cmd) => cmd.execute(),
        }
    }
}

/// Execute the Python CLI inside the Docker container via `docker exec`.
/// `args` are the subcommand path and options forwarded to `cli/main.py`.
pub fn exec_python_cli(args: &[String]) -> anyhow::Result<()> {
    let status = ProcessCommand::new("docker")
        .arg("exec")
        .arg("-i")
        .arg("api")
        .arg("bt")
        .args(args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute docker exec: {}", e))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("Python CLI exited with non-zero status: {}", code);
    }
    Ok(())
}
