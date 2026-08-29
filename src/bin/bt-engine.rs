use backtesting_engine::engine;
use backtesting_engine::infrastructure::logging;

fn main() -> anyhow::Result<()> {
    logging::init()?;
    let cli = engine::build_cli();
    let matches = cli.get_matches();

    let command = engine::parse(matches);
    command.execute()?;

    Ok(())
}
