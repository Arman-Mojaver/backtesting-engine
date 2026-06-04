use backtesting_engine::engine;

fn main() -> anyhow::Result<()> {
    let cli = engine::build_cli();
    let matches = cli.get_matches();

    let command = engine::parse(matches);
    command.execute()?;

    Ok(())
}
