//! Logging initialization.
//!
//! Sets up a [`CombinedLogger`] that writes to **stderr** (for container output)
//! and to the file `logs/bt-engine.log` (for persistent on-disk logs). The log
//! directory is created on init if missing, so the binary also works outside
//! the compose setup, which relies on a pre-mounted `logs/` volume.
//!
//! Log timestamps carry the UTC date and time-of-day (`YYYY-MM-DD HH:MM:SS`),
//! so multi-day batch runs can be correlated by day.
//!
//! The log level is controlled by the `RUST_LOG` environment variable
//! (e.g. `info`, `debug`, `backtesting_engine=debug`). Defaults to `info`
//! when unset.

use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result};
use log::LevelFilter;
use simplelog::{
    format_description, ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode,
    WriteLogger,
};

/// Default log level when `RUST_LOG` is not set.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Log file path, relative to the process working directory.
const LOG_FILE: &str = "logs/bt-engine.log";

/// Initializes the global logger.
///
/// Reads `RUST_LOG` (defaulting to `info`), opens `logs/bt-engine.log` in
/// append/create mode — creating the log directory if it does not exist —
/// and installs a `CombinedLogger` that fans out to both stderr and the
/// file.
///
/// # Errors
/// Returns an error if the log directory or file cannot be created/opened,
/// or if the global logger has already been initialized.
pub fn init() -> Result<()> {
    let level = parse_level(std::env::var("RUST_LOG").ok().as_deref());

    let config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Error)
        .set_max_level(LevelFilter::Error)
        .set_target_level(LevelFilter::Error)
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .build();

    let log_file = open_log_file(Path::new(LOG_FILE))?;

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

/// Opens `path` in append/create mode, creating its parent directory first
/// if it does not exist (std treats an empty parent path as a no-op).
///
/// # Errors
/// Returns an error if the parent directory cannot be created or the file
/// cannot be opened.
fn open_log_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            let parent = parent.display();
            format!("Failed to create log directory: {parent}")
        })?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| {
            let path = path.display();
            format!("Failed to open log file: {path}")
        })
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

    #[test]
    fn open_log_file_creates_missing_parent_directory() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        // Neither `<tmp>/logs/` nor the log file exists yet.
        let log_path = dir.path().join("logs").join("bt-engine.log");

        let file = open_log_file(&log_path).expect("open should create the parent directory");
        drop(file);

        assert!(
            dir.path().join("logs").is_dir(),
            "directory should be created"
        );
        assert!(log_path.is_file(), "log file should be created");
    }

    #[test]
    fn open_log_file_appends_without_truncating() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir creation failed");
        // Parent directory already exists; opening must not truncate it.
        let log_path = dir.path().join("bt-engine.log");

        let mut first = open_log_file(&log_path).expect("initial open");
        writeln!(first, "one").expect("initial write");
        drop(first);

        let mut second = open_log_file(&log_path).expect("re-open");
        writeln!(second, "two").expect("append write");
        drop(second);

        let contents = std::fs::read_to_string(&log_path).expect("read back");
        assert!(
            contents.contains("one\ntwo"),
            "append mode must preserve existing content, got: {contents}"
        );
    }

    #[test]
    fn init_creates_missing_log_dir_and_writes_dated_lines() {
        // A fresh working directory has no `logs/`; `init` must create it
        // instead of failing. The global logger is only ever initialized
        // here, so this is safe under both per-process nextest and
        // single-process `cargo test`.
        const MARKER: &str = "init-smoke-marker-9f17";

        init().expect("init from a directory without logs/");
        // The timestamp (and level/target) are configured to display at the
        // error level, so log at error for the smoke line.
        log::error!("{MARKER}");

        let written = std::fs::read_to_string(LOG_FILE).expect("log file readable");
        let line = written
            .lines()
            .rev()
            .find(|l| l.contains(MARKER))
            .expect("marker line should have been written");
        assert_has_dated_timestamp(line);
    }

    /// Asserts the first two whitespace-separated tokens of `line` are a
    /// `YYYY-MM-DD` date and an `HH:MM:SS` time-of-day.
    fn assert_has_dated_timestamp(line: &str) {
        let mut tokens = line.split_whitespace();
        let date = tokens.next().unwrap_or_default();
        let time = tokens.next().unwrap_or_default();

        assert_eq!(date.len(), 10, "date should be YYYY-MM-DD, got: {line}");
        assert_eq!(
            date.matches('-').count(),
            2,
            "date should be YYYY-MM-DD, got: {line}"
        );
        assert_eq!(
            date.chars().filter(char::is_ascii_digit).count(),
            8,
            "date should be YYYY-MM-DD, got: {line}"
        );

        assert_eq!(time.len(), 8, "time should be HH:MM:SS, got: {line}");
        assert_eq!(
            time.matches(':').count(),
            2,
            "time should be HH:MM:SS, got: {line}"
        );
        assert_eq!(
            time.chars().filter(char::is_ascii_digit).count(),
            6,
            "time should be HH:MM:SS, got: {line}"
        );
    }
}
