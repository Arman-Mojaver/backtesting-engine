use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_strategy_cli() -> Command {
    Command::new("strategy").visible_alias("s")
        .about("Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create").about("Create Strategies").arg(
                Arg::new("instrument")
                    .short('i')
                    .long("instrument")
                    .default_value("EURUSD")
                    .required(true)
                    .help("Instrument"),
            ),
        )
        .subcommand(
            Command::new("create_one")
                .about("Create Strategies for one Money Management Strategy")
                .arg(
                    Arg::new("instrument")
                        .short('i')
                        .long("instrument")
                        .default_value("EURUSD")
                        .required(true)
                        .help("Instrument"),
                )
                .arg(
                    Arg::new("money_management_strategy_id")
                        .short('m')
                        .long("money_management_strategy_id")
                        .required(true)
                        .value_parser(clap::value_parser!(i32))
                        .help("Money Management Strategy ID"),
                )
                .arg(
                    Arg::new("indicator_type")
                        .short('t')
                        .long("indicator_type")
                        .required(true)
                        .help("Indicator Type"),
                ),
        )
        .subcommand(Command::new("delete").about("Delete Strategies"))
        .subcommand(
            Command::new("display_table")
                .about("Display Strategy table")
                .arg(
                    Arg::new("strategy_id")
                        .required(true)
                        .value_parser(clap::value_parser!(i32))
                        .help("Strategy ID"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(ArgAction::SetTrue)
                        .help("Verbose output"),
                ),
        )
        .subcommand(Command::new("terminate").about("Terminate Create Strategies Task"))
        .subcommand(build_strategy_filters_cli())
}

fn build_strategy_filters_cli() -> Command {
    Command::new("filters")
        .about("Strategy Filters CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("custom")
                .about("Apply Custom Filters")
                .arg(
                    Arg::new("instrument")
                        .short('i')
                        .long("instrument")
                        .default_value("EURUSD")
                        .required(true)
                        .help("Instrument"),
                )
                .arg(
                    Arg::new("indicator_type")
                        .short('t')
                        .long("indicator_type")
                        .default_value("macd")
                        .required(true)
                        .help("Indicator Type"),
                )
                .arg(
                    Arg::new("keep_count")
                        .short('k')
                        .long("keep_count")
                        .default_value("20")
                        .value_parser(clap::value_parser!(i32))
                        .help("Keep count"),
                )
                .arg(
                    Arg::new("delta")
                        .short('d')
                        .long("delta")
                        .default_value("1.0")
                        .value_parser(clap::value_parser!(f64))
                        .help("Delta"),
                )
                .arg(
                    Arg::new("max_max_draw_down")
                        .short('m')
                        .long("max_max_draw_down")
                        .default_value("16.0")
                        .value_parser(clap::value_parser!(f64))
                        .help("Max max_draw_down"),
                )
                .arg(
                    Arg::new("graph")
                        .short('g')
                        .long("graph")
                        .action(ArgAction::SetTrue)
                        .help("Generate graph"),
                ),
        )
        .subcommand(
            Command::new("global").about("Apply Global Filters").arg(
                Arg::new("instrument")
                    .short('i')
                    .long("instrument")
                    .default_value("EURUSD")
                    .required(true)
                    .help("Instrument"),
            ),
        )
}

pub enum StrategyCommand {
    Create(CreateStrategy),
    CreateOne(CreateStrategyOne),
    Delete,
    DisplayTable(DisplayStrategyTable),
    Terminate,
    Filters(StrategyFiltersCommand),
}

pub struct CreateStrategy {
    pub instrument: String,
}

pub struct CreateStrategyOne {
    pub instrument: String,
    pub money_management_strategy_id: i32,
    pub indicator_type: String,
}

pub struct DisplayStrategyTable {
    pub strategy_id: i32,
    pub verbose: bool,
}

pub enum StrategyFiltersCommand {
    Custom(ApplyCustomFilters),
    Global(ApplyGlobalFilters),
}

pub struct ApplyCustomFilters {
    pub instrument: String,
    pub indicator_type: String,
    pub keep_count: i32,
    pub delta: f64,
    pub max_max_draw_down: f64,
    pub graph: bool,
}

pub struct ApplyGlobalFilters {
    pub instrument: String,
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("create", sub)) => StrategyCommand::Create(CreateStrategy {
            instrument: sub
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".into()),
        }),
        Some(("create_one", sub)) => StrategyCommand::CreateOne(CreateStrategyOne {
            instrument: sub
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".into()),
            money_management_strategy_id: *sub
                .get_one::<i32>("money_management_strategy_id")
                .unwrap(),
            indicator_type: sub.get_one::<String>("indicator_type").cloned().unwrap(),
        }),
        Some(("delete", _)) => StrategyCommand::Delete,
        Some(("display_table", sub)) => StrategyCommand::DisplayTable(DisplayStrategyTable {
            strategy_id: *sub.get_one::<i32>("strategy_id").unwrap(),
            verbose: sub.get_flag("verbose"),
        }),
        Some(("terminate", _)) => StrategyCommand::Terminate,
        Some(("filters", sub)) => StrategyCommand::Filters(parse_strategy_filters(sub)),
        _ => unreachable!("invalid strategy command"),
    }
}

fn parse_strategy_filters(m: &ArgMatches) -> StrategyFiltersCommand {
    match m.subcommand() {
        Some(("custom", sub)) => StrategyFiltersCommand::Custom(ApplyCustomFilters {
            instrument: sub
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".into()),
            indicator_type: sub
                .get_one::<String>("indicator_type")
                .cloned()
                .unwrap_or_else(|| "macd".into()),
            keep_count: *sub.get_one::<i32>("keep_count").unwrap_or(&20),
            delta: *sub.get_one::<f64>("delta").unwrap_or(&1.0),
            max_max_draw_down: *sub.get_one::<f64>("max_max_draw_down").unwrap_or(&16.0),
            graph: sub.get_flag("graph"),
        }),
        Some(("global", sub)) => StrategyFiltersCommand::Global(ApplyGlobalFilters {
            instrument: sub
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".into()),
        }),
        _ => unreachable!("invalid strategy filters command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create(cmd) => cmd.execute(),
            StrategyCommand::CreateOne(cmd) => cmd.execute(),
            StrategyCommand::Delete => {
                println!("s delete");
                Ok(())
            }
            StrategyCommand::DisplayTable(cmd) => cmd.execute(),
            StrategyCommand::Terminate => {
                println!("s terminate");
                Ok(())
            }
            StrategyCommand::Filters(cmd) => cmd.execute(),
        }
    }
}

impl CreateStrategy {
    fn execute(self) -> anyhow::Result<()> {
        println!("s create --instrument {}", self.instrument);
        Ok(())
    }
}

impl CreateStrategyOne {
    fn execute(self) -> anyhow::Result<()> {
        println!(
            "s create_one --instrument {} --money_management_strategy_id {} --indicator_type {}",
            self.instrument, self.money_management_strategy_id, self.indicator_type
        );
        Ok(())
    }
}

impl DisplayStrategyTable {
    fn execute(self) -> anyhow::Result<()> {
        println!(
            "s display_table --strategy_id {} --verbose {}",
            self.strategy_id, self.verbose
        );
        Ok(())
    }
}

impl StrategyFiltersCommand {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyFiltersCommand::Custom(cmd) => cmd.execute(),
            StrategyFiltersCommand::Global(cmd) => cmd.execute(),
        }
    }
}

impl ApplyCustomFilters {
    fn execute(self) -> anyhow::Result<()> {
        println!(
            "s filters custom --instrument {} --indicator_type {} --keep_count {} --delta {} --max_max_draw_down {} --graph {}",
            self.instrument, self.indicator_type, self.keep_count, self.delta, self.max_max_draw_down, self.graph
        );
        Ok(())
    }
}

impl ApplyGlobalFilters {
    fn execute(self) -> anyhow::Result<()> {
        println!("s filters global --instrument {}", self.instrument);
        Ok(())
    }
}
