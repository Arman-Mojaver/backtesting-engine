use clap::{value_parser, Arg, ArgMatches, Command};

fn build_common_generate_args() -> Vec<Arg> {
    vec![
        Arg::new("count")
            .short('c')
            .long("count")
            .required(true)
            .value_parser(value_parser!(u64))
            .help("Total number of strategies to generate"),
        Arg::new("instruments")
            .long("instruments")
            .short('i')
            .required(false)
            .num_args(1..)
            .default_values(["EURUSD", "USDJPY", "GBPUSD"])
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .help("Instruments to distribute across (partition keys)"),
        Arg::new("mms_count")
            .long("mms-count")
            .short('m')
            .required(false)
            .default_value("10")
            .value_parser(value_parser!(u32))
            .help("Number of money_management_strategy_id values per instrument"),
        Arg::new("max_op_points")
            .long("op-points")
            .short('p')
            .required(false)
            .default_value("1000")
            .value_parser(value_parser!(u32))
            .help("Max operation-point IDs available (each strategy gets a random subset)"),
        Arg::new("seed")
            .long("seed")
            .short('s')
            .required(false)
            .value_parser(value_parser!(u64))
            .help("Random seed for reproducible generation"),
        Arg::new("parquet_dir")
            .long("parquet-dir")
            .required(false)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .help("Override parquet base directory (for running outside Docker)"),
    ]
}

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .visible_alias("s")
        .about("Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create a Strategy")
                .arg(
                    Arg::new("money_management_strategy_id")
                        .short('m')
                        .long("money_management_strategy_id")
                        .required(true)
                        .value_parser(value_parser!(i32))
                        .help("Money management strategy ID"),
                )
                .arg(
                    Arg::new("instrument")
                        .short('i')
                        .long("instrument")
                        .required(false)
                        .default_value("EURUSD")
                        .help("Instrument pair (default: EURUSD)"),
                ),
        )
        .subcommand(
            Command::new("generate")
                .about("Generate purely-random strategies")
                .args(build_common_generate_args()),
        )
}

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
    Generate {
        count: u64,
        instruments: Vec<String>,
        mms_count: u32,
        max_op_points: u32,
        seed: Option<u64>,
        parquet_dir: Option<String>,
    },
}

fn parse_generate(sub_m: &ArgMatches) -> StrategyCommand {
    let count = *sub_m.get_one::<u64>("count").unwrap();
    let instruments: Vec<String> = sub_m
        .get_many::<String>("instruments")
        .unwrap_or_default()
        .cloned()
        .collect();
    let mms_count = *sub_m.get_one::<u32>("mms_count").unwrap();
    let max_op_points = *sub_m.get_one::<u32>("max_op_points").unwrap();
    let seed = sub_m.get_one::<u64>("seed").copied();
    let parquet_dir = sub_m.get_one::<String>("parquet_dir").cloned();

    StrategyCommand::Generate {
        count,
        instruments,
        mms_count,
        max_op_points,
        seed,
        parquet_dir,
    }
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("create", sub_m)) => StrategyCommand::Create {
            money_management_strategy_id: *sub_m
                .get_one::<i32>("money_management_strategy_id")
                .unwrap(),
            instrument: sub_m
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".to_string()),
        },
        Some(("generate", sub_m)) => parse_generate(sub_m),
        _ => unreachable!("invalid strategy command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create {
                money_management_strategy_id,
                instrument,
            } => super::exec_engine_cli(&[
                "strategy".to_string(),
                "create".to_string(),
                "--money_management_strategy_id".to_string(),
                money_management_strategy_id.to_string(),
                "--instrument".to_string(),
                instrument,
            ]),
            StrategyCommand::Generate {
                count,
                instruments,
                mms_count,
                max_op_points,
                seed,
                parquet_dir,
            } => {
                let mut args = vec![
                    "strategy".to_string(),
                    "generate".to_string(),
                    "--count".to_string(),
                    count.to_string(),
                ];
                args.push("--instruments".to_string());
                for inst in &instruments {
                    args.push(inst.clone());
                }

                args.push("--mms-count".to_string());
                args.push(mms_count.to_string());

                args.push("--op-points".to_string());
                args.push(max_op_points.to_string());

                if let Some(s) = seed {
                    args.push("--seed".to_string());
                    args.push(s.to_string());
                }

                if let Some(dir) = parquet_dir {
                    args.push("--parquet-dir".to_string());
                    args.push(dir);
                }

                super::exec_engine_cli(&args)
            }
        }
    }
}
