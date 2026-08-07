use std::collections::HashMap;

use crate::domain::signal_group::SignalGroup;
use crate::infrastructure::redis::{
    delete_signal_groups, list_signal_group_keys, read_signal_groups, signal_groups_key,
    write_signal_groups,
};

use clap::{Arg, ArgAction, ArgMatches, Command};

const REDIS_URL: &str = "redis://redis:6379/0";

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
        let mut conn = connect()?;

        match self {
            RedisCommand::Check {
                instrument,
                verbose,
            } => match instrument {
                Some(inst) => {
                    let key = signal_groups_key(&inst);
                    match read_signal_groups(&mut conn, &key)? {
                        Some(groups) => {
                            if verbose {
                                let json = serde_json::to_string_pretty(&groups)
                                    .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
                                println!("{}", json);
                            } else {
                                println!("Instrument: {}", inst);
                                println!("Key: {}", key);
                                println!("Groups: {}", groups.len());
                                for g in &groups {
                                    println!(
                                        "  id={}, ops={}, longs={}, shorts={}",
                                        g.indicator_id,
                                        g.operation_count,
                                        g.long_signals.len(),
                                        g.short_signals.len()
                                    );
                                }
                            }
                        }
                        None => {
                            println!("Cache: MISS for {}", inst);
                        }
                    }
                }
                None => {
                    let keys = list_signal_group_keys(&mut conn)?;
                    if keys.is_empty() {
                        println!("Cache: EMPTY (no instruments cached)");
                    } else {
                        println!("Cached instruments:");
                        for key in &keys {
                            let inst = key.strip_prefix("signal_groups:").unwrap_or(key);
                            let count = match read_signal_groups(&mut conn, key)? {
                                Some(groups) => groups.len(),
                                None => 0,
                            };
                            println!("  {} — {} group(s)", inst, count);
                        }
                    }
                }
            },

            RedisCommand::Clean { instrument } => {
                let key = signal_groups_key(&instrument);
                delete_signal_groups(&mut conn, &key)?;
                println!("Cache cleared for {}.", instrument);
            }

            RedisCommand::Set => {
                let keys = list_signal_group_keys(&mut conn)?;
                if !keys.is_empty() {
                    anyhow::bail!(
                        "Cache is not empty ({} key(s) found). Use --clean first to clear existing data.",
                        keys.len()
                    );
                }

                let groups = sample_signal_groups();
                let mut by_instrument: HashMap<String, Vec<SignalGroup>> = HashMap::new();
                for g in groups {
                    by_instrument
                        .entry(g.instrument.clone())
                        .or_default()
                        .push(g);
                }

                let mut instruments: Vec<&String> = by_instrument.keys().collect();
                instruments.sort();

                for inst in &instruments {
                    let data = &by_instrument[*inst];
                    let key = signal_groups_key(inst);
                    write_signal_groups(&mut conn, &key, data)?;
                    println!("  {} — {} group(s) written", inst, data.len());
                }
                println!(
                    "Cache populated with {} instrument(s).",
                    by_instrument.len()
                );
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
        // ── EURUSD ──────────────────────────────────────────────
        SignalGroup {
            indicator_id: 101,
            long_signals: vec![1704067200, 1704153600, 1704240000, 1704326400, 1704412800],
            short_signals: vec![1704499200, 1704585600, 1704672000],
            operation_count: 8,
            instrument: "EURUSD".to_string(),
        },
        SignalGroup {
            indicator_id: 102,
            long_signals: vec![1704067200, 1704240000, 1704412800],
            short_signals: vec![1704153600, 1704326400, 1704499200, 1704585600],
            operation_count: 7,
            instrument: "EURUSD".to_string(),
        },
        SignalGroup {
            indicator_id: 103,
            long_signals: vec![1704067200],
            short_signals: vec![],
            operation_count: 1,
            instrument: "EURUSD".to_string(),
        },
        // ── USDJPY ──────────────────────────────────────────────
        SignalGroup {
            indicator_id: 201,
            long_signals: vec![1704067200, 1704153600],
            short_signals: vec![1704240000],
            operation_count: 3,
            instrument: "USDJPY".to_string(),
        },
        SignalGroup {
            indicator_id: 202,
            long_signals: vec![1704326400, 1704412800, 1704499200, 1704585600, 1704672000],
            short_signals: vec![],
            operation_count: 5,
            instrument: "USDJPY".to_string(),
        },
        // ── GBPUSD ──────────────────────────────────────────────
        SignalGroup {
            indicator_id: 301,
            long_signals: vec![1704067200, 1704153600, 1704240000],
            short_signals: vec![1704326400, 1704412800],
            operation_count: 5,
            instrument: "GBPUSD".to_string(),
        },
        SignalGroup {
            indicator_id: 302,
            long_signals: vec![],
            short_signals: vec![1704499200, 1704585600, 1704672000],
            operation_count: 3,
            instrument: "GBPUSD".to_string(),
        },
    ]
}
