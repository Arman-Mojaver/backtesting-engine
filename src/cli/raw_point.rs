use clap::{ArgMatches, Command};

pub fn build_raw_point_cli() -> Command {
    Command::new("raw-point").visible_alias("r")
        .about("Raw Point CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("create").about("Create Raw Points"))
        .subcommand(Command::new("delete").about("Delete Raw Points"))
}

pub enum RawPointCommand {
    Create,
    Delete,
}

pub fn parse_raw_point(m: &ArgMatches) -> RawPointCommand {
    match m.subcommand() {
        Some(("create", _)) => RawPointCommand::Create,
        Some(("delete", _)) => RawPointCommand::Delete,
        _ => unreachable!("invalid raw point command"),
    }
}

impl RawPointCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            RawPointCommand::Create => {
                println!("r create");
                Ok(())
            }
            RawPointCommand::Delete => {
                println!("r delete");
                Ok(())
            }
        }
    }
}
