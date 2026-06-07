use clap::{value_parser, Arg, ArgMatches, Command};

use crate::application::strategy_data::{fetch_strategy_data, StrategyData, StrategyQuery};

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
                    instrument,
                };
                let data = fetch_strategy_data(&query)?;
                print_strategy_data(&data);
                Ok(())
            }
        }
    }
}

/// Print a formatted summary of the given [`StrategyData`] to stdout.
fn print_strategy_data(data: &StrategyData) {
    println!("=== Indicators ({}) ===", data.indicators.len());
    for ind in data.indicators.iter().take(5) {
        println!(
            "  #{} type={} id={} params={:?}",
            ind.id, ind.r#type, ind.identifier, ind.parameters
        );
    }

    println!("=== Resampled Points ({}) ===", data.resampled_points.len());
    for rp in data.resampled_points.iter().take(5) {
        println!(
            "  #{} {} O={} H={} L={} C={} V={} ts={} hlo={:?}",
            rp.id,
            rp.instrument,
            rp.open,
            rp.high,
            rp.low,
            rp.close,
            rp.volume,
            rp.timestamp,
            rp.high_low_order
        );
    }

    let mms_id = data
        .long_operation_points
        .first()
        .map(|op| op.money_management_strategy_id);
    println!(
        "=== Long Operation Points (mms_id={}) ===",
        mms_id.map_or("n/a".to_string(), |id| id.to_string())
    );
    for op in data.long_operation_points.iter().take(5) {
        println!(
            "  #{} {} result={} tp={} sl={} risk={} mms={} ts={}",
            op.id,
            op.instrument,
            op.result,
            op.tp,
            op.sl,
            op.risk,
            op.money_management_strategy_id,
            op.timestamp
        );
    }

    println!(
        "=== Short Operation Points (mms_id={}) ===",
        data.short_operation_points
            .first()
            .map_or("n/a".to_string(), |op| op
                .money_management_strategy_id
                .to_string())
    );
    for op in data.short_operation_points.iter().take(5) {
        println!(
            "  #{} {} result={} tp={} sl={} risk={} mms={} ts={}",
            op.id,
            op.instrument,
            op.result,
            op.tp,
            op.sl,
            op.risk,
            op.money_management_strategy_id,
            op.timestamp
        );
    }

    println!();
    println!("Indicators: {}", data.indicators.len());
    println!("Resampled Points: {}", data.resampled_points.len());
    println!(
        "Long Operation Points: {}",
        data.long_operation_points.len()
    );
    println!(
        "Short Operation Points: {}",
        data.short_operation_points.len()
    );
}
