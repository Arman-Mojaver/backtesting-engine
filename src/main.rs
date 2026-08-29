fn main() -> anyhow::Result<()> {
    backtesting_engine::infrastructure::logging::init()?;
    let cli = backtesting_engine::cli::build_cli();
    let matches = cli.get_matches();
    let command = backtesting_engine::cli::parse(matches);
    command.execute()?;
    Ok(())
}
