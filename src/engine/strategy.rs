use clap::{value_parser, Arg, ArgMatches, Command};

use crate::application::strategy_data::{fetch_strategy_data, StrategyQuery};
use crate::application::strategy_generator::generate_random_strategies;
use crate::config::environment::{parquet_base_dir, Environment};
use crate::domain::validation::combined_validation::{validate_all, CombinedValidationError};
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
}

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .about("Strategy operations")
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
        _ => unreachable!("invalid strategy command"),
    }
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
                );

                let env: Environment = std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string())
                    .parse()
                    .unwrap_or(Environment::Development);
                let base_dir = parquet_base_dir(env);
                let repo = ParquetStrategyRepository::new(base_dir);

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
        }
    }
}
