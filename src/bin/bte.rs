use backtesting_engine::engine;
use backtesting_engine::infrastructure::logging;

/// Mimalloc as the global allocator: the hot path churns Arrow/Parquet buffers
/// across many rayon threads, where it clearly beats the platform default.
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> anyhow::Result<()> {
    logging::init()?;
    let cli = engine::build_cli();
    let matches = cli.get_matches();

    let command = engine::parse(&matches)?;
    command.execute()?;

    Ok(())
}
