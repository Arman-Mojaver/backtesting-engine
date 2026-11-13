mod create;
mod create_loop;

use clap::{Arg, ArgMatches, Command, value_parser};

pub(super) const STRATEGY: &str = "strategy";
const CREATE: &str = "create";
const CREATE_LOOP: &str = "create_loop";
const MMS_ID: &str = "money_management_strategy_id";
const INSTRUMENT: &str = "instrument";

#[derive(Debug, PartialEq, Eq)]
pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
    CreateLoop,
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Create {
                money_management_strategy_id,
                instrument,
            } => create::execute(money_management_strategy_id, &instrument),
            Self::CreateLoop => create_loop::execute(),
        }
    }
}

pub fn build_cli() -> Command {
    Command::new(STRATEGY)
        .about("Strategy operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(CREATE)
                .about("Create a Strategy (fetches data from PostgreSQL)")
                .arg(
                    Arg::new(MMS_ID)
                        .short('m')
                        .long(MMS_ID)
                        .required(true)
                        .value_parser(value_parser!(i32))
                        .help("Money management strategy ID"),
                )
                .arg(
                    Arg::new(INSTRUMENT)
                        .short('i')
                        .long(INSTRUMENT)
                        .default_value("EURUSD")
                        .help("Instrument pair"),
                ),
        )
        .subcommand(
            Command::new(CREATE_LOOP)
                .about("Run idempotent strategy pipeline for all (instrument, mms_id) pairs"),
        )
}

pub fn parse(m: &ArgMatches) -> anyhow::Result<StrategyCommand> {
    match m.subcommand() {
        Some((CREATE, sub_m)) => Ok(StrategyCommand::Create {
            money_management_strategy_id: required_arg(sub_m, MMS_ID)?,
            instrument: required_arg(sub_m, INSTRUMENT)?,
        }),
        Some((CREATE_LOOP, _)) => Ok(StrategyCommand::CreateLoop),
        _ => anyhow::bail!("invalid strategy command"),
    }
}

fn required_arg<T: Clone + Send + Sync + 'static>(m: &ArgMatches, id: &str) -> anyhow::Result<T> {
    m.try_get_one::<T>(id)?
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing required argument '{id}'"))
}
