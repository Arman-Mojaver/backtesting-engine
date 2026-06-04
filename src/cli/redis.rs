use clap::{Arg, ArgMatches, Command};

pub fn build_redis_cli() -> Command {
    Command::new("redis")
        .about("Redis CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("get")
                .about("Read data from Redis")
                .arg(Arg::new("key").help("Redis key to read").num_args(0..=1)),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete Key-Value from Redis")
                .arg(Arg::new("key").required(true).help("Redis key to delete")),
        )
}

pub enum RedisCommand {
    Get(RedisGet),
    Delete(RedisDelete),
}

pub struct RedisGet {
    pub key: Option<String>,
}

pub struct RedisDelete {
    pub key: String,
}

pub fn parse_redis(m: &ArgMatches) -> RedisCommand {
    match m.subcommand() {
        Some(("get", sub)) => RedisCommand::Get(RedisGet {
            key: sub.get_one::<String>("key").cloned(),
        }),
        Some(("delete", sub)) => RedisCommand::Delete(RedisDelete {
            key: sub.get_one::<String>("key").cloned().unwrap(),
        }),
        _ => unreachable!("invalid redis command"),
    }
}

impl RedisCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            RedisCommand::Get(cmd) => cmd.execute(),
            RedisCommand::Delete(cmd) => cmd.execute(),
        }
    }
}


impl RedisGet {
    fn execute(self) -> anyhow::Result<()> {
        let mut args = vec!["redis".to_string(), "get".to_string()];
        if let Some(key) = self.key {
            args.push(key);
        }
        super::exec_python_cli(&args)
    }
}

impl RedisDelete {
    fn execute(self) -> anyhow::Result<()> {
        let args = vec!["redis".to_string(), "delete".to_string(), self.key];
        super::exec_python_cli(&args)
    }
}
