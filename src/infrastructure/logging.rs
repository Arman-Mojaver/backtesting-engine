//! Logging initialization.
//!
//! Sets up a [`CombinedLogger`] that writes to **stderr** (for container output)
//! and to the file `bt-engine.log` (for persistent on-disk logs).
//!
//! The log level is controlled by the `RUST_LOG` environment variable
//! (e.g. `info`, `debug`, `backtesting_engine=debug`). Defaults to `info`
//! when unset.

use std::fs::OpenOptions;

use anyhow::{Context, Result};
use log::LevelFilter;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};

/// Default log level when `RUST_LOG` is not set.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Log file name.
const LOG_FILE: &str = "bt-engine.log";

/// Initializes the global logger.
///
/// Reads `RUST_LOG` (defaulting to `info`), opens `bt-engine.log` in
/// append/create mode, and installs a `CombinedLogger` that fans out to
/// both stderr and the file.
///
/// # Errors
/// Returns an error if the log file cannot be opened or if the global
/// logger has already been initialized.
pub fn init() -> Result<()> {
    let level = parse_level(std::env::var("RUST_LOG").ok().as_deref());

    let config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Error)
        .set_max_level(LevelFilter::Error)
        .set_target_level(LevelFilter::Error)
        .build();

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .with_context(|| format!("Failed to open log file: {LOG_FILE}"))?;

    CombinedLogger::init(vec![
        TermLogger::new(
            level,
            config.clone(),
            TerminalMode::Stderr,
            ColorChoice::Auto,
        ),
        WriteLogger::new(level, config, log_file),
    ])
    .context("Failed to initialize logger")?;

    Ok(())
}

/// Parses a `RUST_LOG`-style string into a `LevelFilter`.
///
/// Accepts a bare level (`"info"`, `"debug"`, …) or a module-targeted
/// spec (`"backtesting_engine=debug"`). Falls back to `Info` on any
/// unrecognised value.
fn parse_level(raw: Option<&str>) -> LevelFilter {
    let raw = match raw {
        Some(s) if !s.is_empty() => s,
        _ => return LevelFilter::Info,
    };

    // Handle "module=level" syntax — take the level after '='.
    let token = raw.split('=').next_back().unwrap_or(raw);

    token.parse::<LevelFilter>().unwrap_or_else(|_| {
        eprintln!("Unrecognised RUST_LOG value '{raw}', defaulting to {DEFAULT_LOG_LEVEL}");
        LevelFilter::Info
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_level_defaults_to_info() {
        assert_eq!(parse_level(None), LevelFilter::Info);
        assert_eq!(parse_level(Some("")), LevelFilter::Info);
    }

    #[test]
    fn parse_level_bare_levels() {
        assert_eq!(parse_level(Some("error")), LevelFilter::Error);
        assert_eq!(parse_level(Some("warn")), LevelFilter::Warn);
        assert_eq!(parse_level(Some("info")), LevelFilter::Info);
        assert_eq!(parse_level(Some("debug")), LevelFilter::Debug);
        assert_eq!(parse_level(Some("trace")), LevelFilter::Trace);
    }

    #[test]
    fn parse_level_module_targeted() {
        assert_eq!(
            parse_level(Some("backtesting_engine=debug")),
            LevelFilter::Debug
        );
    }

    #[test]
    fn parse_level_case_insensitive() {
        assert_eq!(parse_level(Some("INFO")), LevelFilter::Info);
        assert_eq!(parse_level(Some("Debug")), LevelFilter::Debug);
    }

    #[test]
    fn parse_level_garbage_falls_back_to_info() {
        assert_eq!(parse_level(Some("nonsense")), LevelFilter::Info);
    }
}
