use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_redis_cli() -> Command {
    Command::new("redis")
        .about("Redis signal groups cache operations")
        .arg(
            Arg::new("instrument")
                .short('i')
                .long("instrument")
                .value_name("SYMBOL")
                .help("Instrument symbol (e.g., EURUSD, USDJPY)"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Full JSON dump (requires --instrument)")
                .requires("instrument")
                .conflicts_with_all(["clean", "set"]),
        )
        .arg(
            Arg::new("clean")
                .long("clean")
                .action(ArgAction::SetTrue)
                .help("Delete cached signal groups for the specified instrument")
                .requires("instrument")
                .conflicts_with_all(["verbose", "set"]),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .action(ArgAction::SetTrue)
                .help("Populate cache with sample data for EURUSD, USDJPY, GBPUSD (only if empty)")
                .conflicts_with_all(["verbose", "clean"]),
        )
}

pub enum RedisCommand {
    Check {
        instrument: Option<String>,
        verbose: bool,
    },
    Clean {
        instrument: String,
    },
    Set,
}

pub fn parse_redis(m: &ArgMatches) -> RedisCommand {
    if m.get_flag("clean") {
        RedisCommand::Clean {
            instrument: m
                .get_one::<String>("instrument")
                .cloned()
                .expect("--clean requires --instrument"),
        }
    } else if m.get_flag("set") {
        RedisCommand::Set
    } else {
        RedisCommand::Check {
            instrument: m.get_one::<String>("instrument").cloned(),
            verbose: m.get_flag("verbose"),
        }
    }
}

impl RedisCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        let mut args = vec!["redis".to_string()];
        match self {
            RedisCommand::Clean { instrument } => {
                args.push("--clean".to_string());
                args.push("--instrument".to_string());
                args.push(instrument);
            }
            RedisCommand::Set => args.push("--set".to_string()),
            RedisCommand::Check {
                instrument,
                verbose,
            } => {
                if let Some(inst) = instrument {
                    args.push("--instrument".to_string());
                    args.push(inst);
                }
                if verbose {
                    args.push("--verbose".to_string());
                }
            }
        }
        super::exec_engine_cli(&args)
    }
}
