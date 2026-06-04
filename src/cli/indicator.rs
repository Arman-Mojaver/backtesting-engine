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
                    .arg(
                        Arg::new("slow_n_range")
                            .short('s')
                            .long("slow_n_range")
                            .num_args(2)
                            .value_parser(clap::value_parser!(i32))
                            .help("Slow n range"),
                    )
                    .arg(
                        Arg::new("fast_n_range")
                            .short('f')
                            .long("fast_n_range")
                            .num_args(2)
                            .value_parser(clap::value_parser!(i32))
                            .help("Fast n range"),
                    )
                    .arg(
                        Arg::new("count")
                            .short('c')
                            .long("count")
                            .default_value("0")
                            .value_parser(clap::value_parser!(i32))
                            .help("Create Random MACD indicators"),
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
                    .arg(
                        Arg::new("n_range")
                            .short('n')
                            .long("n_range")
                            .num_args(2)
                            .value_parser(clap::value_parser!(i32))
                            .help("n range"),
                    )
                    .arg(
                        Arg::new("high_threshold_range")
                            .short('u')
                            .long("high_threshold_range")
                            .num_args(3)
                            .value_parser(clap::value_parser!(i32))
                            .help("High Threshold Range"),
                    )
                    .arg(
                        Arg::new("low_threshold_range")
                            .short('w')
                            .long("low_threshold_range")
                            .num_args(3)
                            .value_parser(clap::value_parser!(i32))
                            .help("Low Threshold Range"),
                    )
                    .arg(
                        Arg::new("count")
                            .short('c')
                            .long("count")
                            .default_value("0")
                            .value_parser(clap::value_parser!(i32))
                            .help("Create Random RSI indicators"),
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

    Command::new("indicator").visible_alias("i")
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
        .subcommand(Command::new("delete_unused").about("Delete Unused Indicators"))
}

pub enum IndicatorCommand {
    Create(CreateIndicator),
    Delete(DeleteIndicators),
    DeleteUnused,
}

pub enum CreateIndicator {
    Macd(Macd),
    Rsi(Rsi),
}

pub struct Macd {
    pub slow_n_range: Option<(i32, i32)>,
    pub fast_n_range: Option<(i32, i32)>,
    pub count: i32,
    pub example: bool,
}

pub struct Rsi {
    pub n_range: Option<(i32, i32)>,
    pub high_threshold_range: Option<(i32, i32, i32)>,
    pub low_threshold_range: Option<(i32, i32, i32)>,
    pub count: i32,
    pub example: bool,
}

pub struct DeleteIndicators {
    pub identifiers: Vec<String>,
}

pub fn parse_indicator(m: &ArgMatches) -> IndicatorCommand {
    match m.subcommand() {
        Some(("create", sub)) => IndicatorCommand::Create(parse_create(sub)),
        Some(("delete", sub)) => IndicatorCommand::Delete(DeleteIndicators {
            identifiers: sub
                .get_many::<String>("identifiers")
                .unwrap_or_default()
                .map(|s| s.to_owned())
                .collect(),
        }),
        Some(("delete_unused", _)) => IndicatorCommand::DeleteUnused,
        _ => unreachable!("invalid indicator command"),
    }
}

fn parse_create(m: &ArgMatches) -> CreateIndicator {
    match m.subcommand() {
        Some(("macd", sub)) => CreateIndicator::Macd(Macd {
            slow_n_range: sub.get_many::<i32>("slow_n_range").map(|v| {
                let xs: Vec<i32> = v.copied().collect();
                (xs[0], xs[1])
            }),
            fast_n_range: sub.get_many::<i32>("fast_n_range").map(|v| {
                let xs: Vec<i32> = v.copied().collect();
                (xs[0], xs[1])
            }),
            count: *sub.get_one::<i32>("count").unwrap_or(&0),
            example: sub.get_flag("example"),
        }),
        Some(("rsi", sub)) => CreateIndicator::Rsi(Rsi {
            n_range: sub.get_many::<i32>("n_range").map(|v| {
                let xs: Vec<i32> = v.copied().collect();
                (xs[0], xs[1])
            }),
            high_threshold_range: sub.get_many::<i32>("high_threshold_range").map(|v| {
                let xs: Vec<i32> = v.copied().collect();
                (xs[0], xs[1], xs[2])
            }),
            low_threshold_range: sub.get_many::<i32>("low_threshold_range").map(|v| {
                let xs: Vec<i32> = v.copied().collect();
                (xs[0], xs[1], xs[2])
            }),
            count: *sub.get_one::<i32>("count").unwrap_or(&0),
            example: sub.get_flag("example"),
        }),
        _ => unreachable!("invalid indicator create subcommand"),
    }
}

impl IndicatorCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            IndicatorCommand::Create(cmd) => cmd.execute(),
            IndicatorCommand::Delete(cmd) => cmd.execute(),
            IndicatorCommand::DeleteUnused => {
                println!("i delete_unused");
                Ok(())
            }
        }
    }
}

impl CreateIndicator {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            CreateIndicator::Macd(cmd) => cmd.execute(),
            CreateIndicator::Rsi(cmd) => cmd.execute(),
        }
    }
}

impl Macd {
    fn execute(self) -> anyhow::Result<()> {
        println!(
            "i create macd --slow_n_range {:?} --fast_n_range {:?} --count {} --example {}",
            self.slow_n_range, self.fast_n_range, self.count, self.example
        );
        Ok(())
    }
}

impl Rsi {
    fn execute(self) -> anyhow::Result<()> {
        println!(
            "i create rsi --n_range {:?} --high_threshold_range {:?} --low_threshold_range {:?} --count {} --example {}",
            self.n_range, self.high_threshold_range, self.low_threshold_range, self.count, self.example
        );
        Ok(())
    }
}

impl DeleteIndicators {
    fn execute(self) -> anyhow::Result<()> {
        println!("i delete identifiers={:?}", self.identifiers);
        Ok(())
    }
}
