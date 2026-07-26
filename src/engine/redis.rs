use crate::domain::signal_group::SignalGroup;
use crate::infrastructure::redis::{delete_signal_groups, read_signal_groups, write_signal_groups};

use clap::{Arg, ArgAction, ArgMatches, Command};

const REDIS_URL: &str = "redis://redis:6379/0";

pub fn build_redis_cli() -> Command {
    Command::new("redis")
        .about("Redis signal groups cache operations")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Verbose HIT/MISS dump showing all signal groups data")
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
        let mut conn = connect()?;

        match self {
            RedisCommand::Check { verbose } => match read_signal_groups(&mut conn)? {
                Some(groups) => {
                    println!("Cache: HIT");
                    if verbose {
                        let json = serde_json::to_string_pretty(&groups)
                            .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
                        println!("{}", json);
                    } else {
                        let mut ids: Vec<_> = groups.iter().map(|g| g.indicator_id).collect();
                        ids.sort();
                        println!("Cached indicator IDs: {:?}", ids);
                    }
                }
                None => {
                    println!("Cache: MISS");
                }
            },
            RedisCommand::Clean => {
                delete_signal_groups(&mut conn)?;
                println!("Cache cleared.");
            }
            RedisCommand::Set => {
                let groups = sample_signal_groups();
                write_signal_groups(&mut conn, &groups)?;
                println!("Cache set with {} indicator(s).", groups.len());
            }
        }

        Ok(())
    }
}

fn connect() -> anyhow::Result<redis::Connection> {
    let client = redis::Client::open(REDIS_URL)
        .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
    let conn = client
        .get_connection()
        .map_err(|e| anyhow::anyhow!("Failed to connect to Redis at {}: {}", REDIS_URL, e))?;
    Ok(conn)
}

fn sample_signal_groups() -> Vec<SignalGroup> {
    vec![
        SignalGroup {
            indicator_id: 1,
            long_signals: vec![1704067200, 1704153600, 1704240000, 1704326400, 1704412800],
            short_signals: vec![1704499200, 1704585600, 1704672000],
            operation_count: 8,
            instrument: "EURUSD".to_string(),
        },
        SignalGroup {
            indicator_id: 2,
            long_signals: vec![1704067200, 1704240000, 1704412800],
            short_signals: vec![1704153600, 1704326400, 1704499200, 1704585600],
            operation_count: 7,
            instrument: "EURUSD".to_string(),
        },
        SignalGroup {
            indicator_id: 3,
            long_signals: vec![1704067200],
            short_signals: vec![],
            operation_count: 1,
            instrument: "EURUSD".to_string(),
        },
    ]
}
