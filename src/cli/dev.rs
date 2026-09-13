use clap::{Arg, ArgMatches, Command};

pub fn build_dev_cli() -> Command {
    Command::new("dev")
        .about("Dev tools CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create_indicators")
                .about("Create Indicators")
                .arg(
                    Arg::new("count")
                        .short('c')
                        .long("count")
                        .default_value("10000")
                        .value_parser(clap::value_parser!(i32))
                        .help("Create indicators"),
                ),
        )
}

pub enum DevCommand {
    CreateIndicators(CreateIndicators),
}

pub struct CreateIndicators {
    pub count: i32,
}

pub fn parse_dev(m: &ArgMatches) -> anyhow::Result<DevCommand> {
    match m.subcommand() {
        Some(("create_indicators", sub)) => Ok(DevCommand::CreateIndicators(CreateIndicators {
            count: *sub.get_one::<i32>("count").unwrap_or(&10000),
        })),
        _ => anyhow::bail!("invalid dev command"),
    }
}

impl DevCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            DevCommand::CreateIndicators(cmd) => cmd.execute(),
        }
    }
}

impl CreateIndicators {
    fn execute(self) -> anyhow::Result<()> {
        let args = vec![
            "dev".to_string(),
            "create_indicators".to_string(),
            "--count".to_string(),
            self.count.to_string(),
        ];
        super::exec_python_cli(&args)
    }
}
