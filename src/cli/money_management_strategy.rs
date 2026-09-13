use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_money_management_strategy_cli() -> Command {
    Command::new("money-management-strategy")
        .visible_alias("mms")
        .about("Money Management Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create multiple Money Management Strategies")
                .trailing_var_arg(true)
                .arg(
                    Arg::new("args")
                        .num_args(0..)
                        .allow_hyphen_values(true)
                        .help("options forwarded to Python CLI"),
                )
                .arg(
                    Arg::new("example")
                        .short('e')
                        .long("example")
                        .action(ArgAction::SetTrue)
                        .help("Display an example"),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete multiple Money Management Strategies")
                .arg(
                    Arg::new("identifiers")
                        .num_args(0..)
                        .help("Money Management Strategy identifiers to delete"),
                ),
        )
}

pub enum MoneyManagementStrategyCommand {
    Create(CreateMoneyManagementStrategy),
    Delete(DeleteMoneyManagementStrategies),
}

pub struct CreateMoneyManagementStrategy {
    pub args: Vec<String>,
    pub example: bool,
}

pub struct DeleteMoneyManagementStrategies {
    pub identifiers: Vec<String>,
}

pub fn parse_money_management_strategy(
    m: &ArgMatches,
) -> anyhow::Result<MoneyManagementStrategyCommand> {
    match m.subcommand() {
        Some(("create", sub)) => Ok(MoneyManagementStrategyCommand::Create(
            CreateMoneyManagementStrategy {
                args: sub
                    .get_many::<String>("args")
                    .unwrap_or_default()
                    .map(std::borrow::ToOwned::to_owned)
                    .collect(),
                example: sub.get_flag("example"),
            },
        )),
        Some(("delete", sub)) => Ok(MoneyManagementStrategyCommand::Delete(
            DeleteMoneyManagementStrategies {
                identifiers: sub
                    .get_many::<String>("identifiers")
                    .unwrap_or_default()
                    .map(std::borrow::ToOwned::to_owned)
                    .collect(),
            },
        )),
        _ => anyhow::bail!("invalid mms command"),
    }
}

impl MoneyManagementStrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            MoneyManagementStrategyCommand::Create(cmd) => cmd.execute(),
            MoneyManagementStrategyCommand::Delete(cmd) => cmd.execute(),
        }
    }
}

impl CreateMoneyManagementStrategy {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec![
            "money-management-strategy".to_string(),
            "create".to_string(),
        ];
        if self.example {
            args.push("--example".to_string());
        }
        args.extend(self.args);
        super::exec_python_cli(&args)
    }
}

impl DeleteMoneyManagementStrategies {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec![
            "money-management-strategy".to_string(),
            "delete".to_string(),
        ];
        args.extend(self.identifiers);
        super::exec_python_cli(&args)
    }
}
