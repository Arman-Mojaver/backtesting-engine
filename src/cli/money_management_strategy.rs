use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_money_management_strategy_cli() -> Command {
    Command::new("money-management-strategy").visible_alias("mms")
        .about("Money Management Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create multiple Money Management Strategies")
                .arg(
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .default_value("atr")
                        .help("Money Management Strategy type"),
                )
                .arg(
                    Arg::new("tp_multiplier_range")
                        .short('p')
                        .long("tp_multiplier_range")
                        .num_args(2)
                        .value_parser(clap::value_parser!(f64))
                        .help("TP Multiplier range"),
                )
                .arg(
                    Arg::new("sl_multiplier_range")
                        .short('s')
                        .long("sl_multiplier_range")
                        .num_args(2)
                        .value_parser(clap::value_parser!(f64))
                        .help("SL Multiplier range"),
                )
                .arg(
                    Arg::new("atr_parameter_range")
                        .short('a')
                        .long("atr_parameter_range")
                        .num_args(2)
                        .value_parser(clap::value_parser!(i32))
                        .help("ATR parameter range"),
                )
                .arg(
                    Arg::new("risk_percentage_range")
                        .short('r')
                        .long("risk_percentage_range")
                        .num_args(2)
                        .value_parser(clap::value_parser!(i32))
                        .help("Risk parameter range"),
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
                        .help("Strategy identifiers to delete"),
                ),
        )
}

pub enum MoneyManagementStrategyCommand {
    Create(CreateMoneyManagementStrategy),
    Delete(DeleteMoneyManagementStrategies),
}

pub struct CreateMoneyManagementStrategy {
    pub type_: String,
    pub tp_multiplier_range: Option<(f64, f64)>,
    pub sl_multiplier_range: Option<(f64, f64)>,
    pub atr_parameter_range: Option<(i32, i32)>,
    pub risk_percentage_range: Option<(i32, i32)>,
    pub example: bool,
}

pub struct DeleteMoneyManagementStrategies {
    pub identifiers: Vec<String>,
}

pub fn parse_money_management_strategy(m: &ArgMatches) -> MoneyManagementStrategyCommand {
    match m.subcommand() {
        Some(("create", sub)) => {
            MoneyManagementStrategyCommand::Create(CreateMoneyManagementStrategy {
                type_: sub
                    .get_one::<String>("type")
                    .cloned()
                    .unwrap_or_else(|| "atr".into()),
                tp_multiplier_range: sub.get_many::<f64>("tp_multiplier_range").map(|v| {
                    let xs: Vec<f64> = v.copied().collect();
                    (xs[0], xs[1])
                }),
                sl_multiplier_range: sub.get_many::<f64>("sl_multiplier_range").map(|v| {
                    let xs: Vec<f64> = v.copied().collect();
                    (xs[0], xs[1])
                }),
                atr_parameter_range: sub.get_many::<i32>("atr_parameter_range").map(|v| {
                    let xs: Vec<i32> = v.copied().collect();
                    (xs[0], xs[1])
                }),
                risk_percentage_range: sub.get_many::<i32>("risk_percentage_range").map(|v| {
                    let xs: Vec<i32> = v.copied().collect();
                    (xs[0], xs[1])
                }),
                example: sub.get_flag("example"),
            })
        }
        Some(("delete", sub)) => {
            MoneyManagementStrategyCommand::Delete(DeleteMoneyManagementStrategies {
                identifiers: sub
                    .get_many::<String>("identifiers")
                    .unwrap_or_default()
                    .map(|s| s.to_owned())
                    .collect(),
            })
        }
        _ => unreachable!("invalid mms command"),
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
        println!(
            "mms create --type {} --tp_multiplier_range {:?} --sl_multiplier_range {:?} --atr_parameter_range {:?} --risk_percentage_range {:?} --example {}",
            self.type_, self.tp_multiplier_range, self.sl_multiplier_range, self.atr_parameter_range, self.risk_percentage_range, self.example
        );
        Ok(())
    }
}

impl DeleteMoneyManagementStrategies {
    fn execute(self) -> anyhow::Result<()> {
        println!("mms delete identifiers={:?}", self.identifiers);
        Ok(())
    }
}
