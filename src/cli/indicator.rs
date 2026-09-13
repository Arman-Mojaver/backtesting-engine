use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_indicator_cli() -> Command {
    fn build_create_cli() -> Command {
        Command::new("create")
            .about("Create Indicators")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(
                Command::new("macd")
                    .about("Create MACD Indicators")
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
                Command::new("rsi")
                    .about("Create RSI Indicators")
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
    }

    Command::new("indicator")
        .visible_alias("i")
        .about("Indicator CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(build_create_cli())
        .subcommand(
            Command::new("delete")
                .about("Delete multiple Indicators")
                .arg(
                    Arg::new("identifiers")
                        .num_args(0..)
                        .help("Indicator identifiers to delete"),
                ),
        )
}

pub enum IndicatorCommand {
    Create(CreateIndicator),
    Delete(DeleteIndicators),
}

pub enum CreateIndicator {
    Macd(Macd),
    Rsi(Rsi),
}

pub struct Macd {
    pub args: Vec<String>,
    pub example: bool,
}

pub struct Rsi {
    pub args: Vec<String>,
    pub example: bool,
}

pub struct DeleteIndicators {
    pub identifiers: Vec<String>,
}

pub fn parse_indicator(m: &ArgMatches) -> anyhow::Result<IndicatorCommand> {
    match m.subcommand() {
        Some(("create", sub)) => Ok(IndicatorCommand::Create(parse_create(sub)?)),
        Some(("delete", sub)) => Ok(IndicatorCommand::Delete(DeleteIndicators {
            identifiers: sub
                .get_many::<String>("identifiers")
                .unwrap_or_default()
                .map(std::borrow::ToOwned::to_owned)
                .collect(),
        })),
        _ => anyhow::bail!("invalid indicator command"),
    }
}

fn parse_create(m: &ArgMatches) -> anyhow::Result<CreateIndicator> {
    match m.subcommand() {
        Some(("macd", sub)) => Ok(CreateIndicator::Macd(Macd {
            args: sub
                .get_many::<String>("args")
                .unwrap_or_default()
                .map(std::borrow::ToOwned::to_owned)
                .collect(),
            example: sub.get_flag("example"),
        })),
        Some(("rsi", sub)) => Ok(CreateIndicator::Rsi(Rsi {
            args: sub
                .get_many::<String>("args")
                .unwrap_or_default()
                .map(std::borrow::ToOwned::to_owned)
                .collect(),
            example: sub.get_flag("example"),
        })),
        _ => anyhow::bail!("invalid indicator create subcommand"),
    }
}

impl IndicatorCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(),
            Self::Delete(cmd) => cmd.execute(),
        }
    }
}

impl CreateIndicator {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Macd(cmd) => cmd.execute(),
            Self::Rsi(cmd) => cmd.execute(),
        }
    }
}

impl Macd {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec![
            "indicator".to_string(),
            "create".to_string(),
            "macd".to_string(),
        ];
        if self.example {
            args.push("--example".to_string());
        }
        args.extend(self.args);
        super::exec_python_cli(&args)
    }
}

impl Rsi {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec![
            "indicator".to_string(),
            "create".to_string(),
            "rsi".to_string(),
        ];
        if self.example {
            args.push("--example".to_string());
        }
        args.extend(self.args);
        super::exec_python_cli(&args)
    }
}

impl DeleteIndicators {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec!["indicator".to_string(), "delete".to_string()];
        args.extend(self.identifiers);
        super::exec_python_cli(&args)
    }
}
