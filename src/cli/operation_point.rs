use clap::{Arg, ArgMatches, Command};

pub fn build_operation_point_cli() -> Command {
    Command::new("operation-point").visible_alias("o")
        .about("Operation Point CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("align")
                .about("Align Points")
                .arg(Arg::new("instrument").required(true).help("Instrument")),
        )
        .subcommand(Command::new("create").about("Create Operation Points"))
        .subcommand(Command::new("delete").about("Delete multiple Operation Points"))
}

pub enum OperationPointCommand {
    Align(AlignOperationPoint),
    Create,
    Delete,
}

pub struct AlignOperationPoint {
    pub instrument: String,
}

pub fn parse_operation_point(m: &ArgMatches) -> OperationPointCommand {
    match m.subcommand() {
        Some(("align", sub)) => OperationPointCommand::Align(AlignOperationPoint {
            instrument: sub.get_one::<String>("instrument").cloned().unwrap(),
        }),
        Some(("create", _)) => OperationPointCommand::Create,
        Some(("delete", _)) => OperationPointCommand::Delete,
        _ => unreachable!("invalid operation point command"),
    }
}

impl OperationPointCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            OperationPointCommand::Align(cmd) => cmd.execute(),
            OperationPointCommand::Create => {
                println!("o create");
                Ok(())
            }
            OperationPointCommand::Delete => {
                println!("o delete");
                Ok(())
            }
        }
    }
}

impl AlignOperationPoint {
    fn execute(self) -> anyhow::Result<()> {
        println!("o align instrument={}", self.instrument);
        Ok(())
    }
}
