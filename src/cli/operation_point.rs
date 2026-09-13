use clap::{Arg, ArgMatches, Command};

pub fn build_operation_point_cli() -> Command {
    Command::new("operation-point")
        .visible_alias("o")
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

pub fn parse_operation_point(m: &ArgMatches) -> anyhow::Result<OperationPointCommand> {
    match m.subcommand() {
        Some(("align", sub)) => Ok(OperationPointCommand::Align(AlignOperationPoint {
            instrument: sub
                .get_one::<String>("instrument")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing required argument 'instrument'"))?,
        })),
        Some(("create", _)) => Ok(OperationPointCommand::Create),
        Some(("delete", _)) => Ok(OperationPointCommand::Delete),
        _ => anyhow::bail!("invalid operation point command"),
    }
}

impl OperationPointCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            OperationPointCommand::Align(cmd) => cmd.execute(),
            OperationPointCommand::Create => {
                super::exec_python_cli(&["operation-point".to_string(), "create".to_string()])
            }
            OperationPointCommand::Delete => {
                super::exec_python_cli(&["operation-point".to_string(), "delete".to_string()])
            }
        }
    }
}

impl AlignOperationPoint {
    fn execute(self) -> anyhow::Result<()> {
        let args = vec![
            "operation-point".to_string(),
            "align".to_string(),
            self.instrument,
        ];
        super::exec_python_cli(&args)
    }
}
