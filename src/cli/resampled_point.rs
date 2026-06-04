use clap::{ArgMatches, Command};

pub fn build_resampled_point_cli() -> Command {
    Command::new("resampled-point")
        .visible_alias("rr")
        .about("Resampled Point CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("create").about("Create Resampled Points"))
        .subcommand(Command::new("delete").about("Delete Resampled Points"))
}

pub enum ResampledPointCommand {
    Create,
    Delete,
}

pub fn parse_resampled_point(m: &ArgMatches) -> ResampledPointCommand {
    match m.subcommand() {
        Some(("create", _)) => ResampledPointCommand::Create,
        Some(("delete", _)) => ResampledPointCommand::Delete,
        _ => unreachable!("invalid resampled point command"),
    }
}

impl ResampledPointCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            ResampledPointCommand::Create => {
                super::exec_python_cli(&["rr".to_string(), "create".to_string()])
            }
            ResampledPointCommand::Delete => {
                super::exec_python_cli(&["rr".to_string(), "delete".to_string()])
            }
        }
    }
}
