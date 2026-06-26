use clap::{value_parser, Arg, ArgMatches, Command};

use crate::application::strategy_data::{fetch_strategy_data, StrategyQuery};
use crate::application::strategy_generator::{
    generate_random_strategies, generate_random_strategies_bulk,
};
use crate::config::environment::{parquet_base_dir, Environment};
use crate::domain::validation::combined_validation::{validate_all, CombinedValidationError};
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

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

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .about("Strategy operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create a Strategy (fetches data from PostgreSQL)")
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
                .arg(
                    Arg::new("count")
                        .short('c')
                        .long("count")
                        .required(true)
                        .value_parser(value_parser!(u64))
                        .help("Total number of strategies to generate"),
                )
                .arg(
                    Arg::new("instruments")
                        .long("instruments")
                        .short('i')
                        .required(false)
                        .num_args(1..)
                        .default_values(["EURUSD", "USDJPY", "GBPUSD"])
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .help("Instruments to distribute across (partition keys)"),
                )
                .arg(
                    Arg::new("mms_count")
                        .long("mms-count")
                        .short('m')
                        .required(false)
                        .default_value("10")
                        .value_parser(value_parser!(u32))
                        .help("Number of money_management_strategy_id values per instrument"),
                )
                .arg(
                    Arg::new("max_op_points")
                        .long("op-points")
                        .short('p')
                        .required(false)
                        .default_value("1000")
                        .value_parser(value_parser!(u32))
                        .help("Max operation-point IDs available (each strategy gets a random subset)"),
                )
                .arg(
                    Arg::new("seed")
                        .long("seed")
                        .short('s')
                        .required(false)
                        .value_parser(value_parser!(u64))
                        .help("Random seed for reproducible generation"),
                )
                .arg(
                    Arg::new("parquet_dir")
                        .long("parquet-dir")
                        .required(false)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .help("Override parquet base directory (for running outside Docker)"),
                ),
        )
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
        Some(("generate", sub_m)) => {
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
        _ => unreachable!("invalid strategy command"),
    }
}

fn resolve_environment() -> Environment {
    std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .parse()
        .unwrap_or(Environment::Development)
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create {
                money_management_strategy_id,
                instrument,
            } => {
                let query = StrategyQuery {
                    money_management_strategy_id,
                    instrument: instrument.clone(),
                };

                eprintln!("Fetching data from database ...");
                let data = fetch_strategy_data(&query)?;

                eprintln!("Validating data ...");
                validate_all(
                    &data.long_operation_points,
                    &data.short_operation_points,
                    &data.indicators,
                    &data.resampled_points,
                    money_management_strategy_id,
                )
                .map_err(|e: CombinedValidationError| {
                    anyhow::anyhow!("Validation failed: {:?}", e)
                })?;

                eprintln!("Generating random strategies ...");
                let strategies = generate_random_strategies(
                    &data.indicators,
                    &data.long_operation_points,
                    &data.short_operation_points,
                    money_management_strategy_id,
                );

                let env = resolve_environment();
                let base_dir = parquet_base_dir(env);
                let repo = ParquetStrategyRepository::new(base_dir.clone());

                eprintln!("Writing {} strategies to parquet ...", strategies.len());
                repo.write(&instrument, money_management_strategy_id, &strategies)?;

                eprintln!(
                    "Done. Wrote {} strategies for {} (mms_id={})",
                    strategies.len(),
                    instrument,
                    money_management_strategy_id,
                );
                Ok(())
            }
            StrategyCommand::Generate {
                count,
                instruments,
                mms_count,
                max_op_points,
                seed,
                parquet_dir,
            } => {
                let base_dir = match parquet_dir {
                    Some(dir) => dir,
                    None => parquet_base_dir(resolve_environment()),
                };
                let repo = ParquetStrategyRepository::new(base_dir.clone());

                let partitions = generate_random_strategies_bulk(
                    count as usize,
                    &instruments,
                    mms_count as usize,
                    max_op_points as usize,
                    seed,
                );

                let total_strategies: usize = partitions.iter().map(|(_, _, s)| s.len()).sum();
                let total_partitions = partitions.len();

                eprintln!(
                    "Generating {} strategies across {} partitions ...",
                    total_strategies, total_partitions,
                );

                for (instrument, mms_id, strategies) in &partitions {
                    repo.write(instrument, *mms_id, strategies)?;
                }

                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("  Generate complete");
                println!("  Base dir:       {}", base_dir);
                println!("  Total rows:     {}", total_strategies);
                println!("  Partitions:     {}", total_partitions);
                for (inst, mms, strs) in &partitions {
                    println!("    {}/mms_id={}: {} rows", inst, mms, strs.len());
                }
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                Ok(())
            }
        }
    }
}
