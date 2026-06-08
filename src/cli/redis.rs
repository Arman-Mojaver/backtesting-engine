use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn build_redis_cli() -> Command {
    Command::new("redis")
        .about("Redis signal groups cache operations")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Verbose HIT/MISS dump")
                .conflicts_with_all(["clean", "set"]),
        )
        .arg(
            Arg::new("clean")
                .long("clean")
                .action(ArgAction::SetTrue)
                .help("Delete cached signal groups")
                .conflicts_with_all(["verbose", "set"]),
        )
        .arg(
            Arg::new("set")
                .long("set")
                .action(ArgAction::SetTrue)
                .help("Write sample signal groups to cache")
                .conflicts_with_all(["verbose", "clean"]),
        )
}

pub enum RedisCommand {
    Check { verbose: bool },
    Clean,
    Set,
}

pub fn parse_redis(m: &ArgMatches) -> RedisCommand {
    if m.get_flag("clean") {
        RedisCommand::Clean
    } else if m.get_flag("set") {
        RedisCommand::Set
    } else {
        RedisCommand::Check {
            verbose: m.get_flag("verbose"),
        }
    }
}

impl RedisCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        let mut args = vec!["redis".to_string()];
        match self {
            RedisCommand::Clean => args.push("--clean".to_string()),
            RedisCommand::Set => args.push("--set".to_string()),
            RedisCommand::Check { verbose } => {
                if verbose {
                    args.push("--verbose".to_string());
                }
            }
        }
        super::exec_engine_cli(&args)
    }
}
