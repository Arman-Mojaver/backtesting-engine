use std::process::ExitCode;

use backtesting_engine::engine;
use backtesting_engine::infrastructure::logging;
use clap::ArgMatches;

/// Mimalloc as the global allocator: the hot path churns Arrow/Parquet buffers
/// across many rayon threads, where it clearly beats the platform default.
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    let matches = engine::build_cli().get_matches();

    if let Err(e) = logging::init() {
        eprintln!("Error: {e:#}");
        return ExitCode::FAILURE;
    }

    match run(&matches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(matches: &ArgMatches) -> anyhow::Result<()> {
    engine::parse(matches)?.execute()
}
