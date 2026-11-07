//! Logging initialization.
//!
//! stderr captures container output and `logs/bte.log` persists it.
//! The directory is created on init so the binary also works outside the
//! compose setup, which pre-mounts a `logs/` volume.

use std::fs::{self, File, OpenOptions};
use std::path::Path;

use anyhow::{Context, Result};
use log::LevelFilter;
use simplelog::{
    format_description, ColorChoice, CombinedLogger, Config, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};

const DEFAULT_LEVEL: LevelFilter = LevelFilter::Info;

pub const LOG_FILE: &str = "logs/bte.log";

pub fn init(rust_log: Option<&str>, log_file: &Path) -> Result<()> {
    let rust_log = rust_log.unwrap_or_default();
    let parsed_level = parse_level(rust_log);
    let level = parsed_level.unwrap_or(DEFAULT_LEVEL);
    let config = log_config();
    let log_file = open_log_file(log_file)?;

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

    if parsed_level.is_none() && !rust_log.is_empty() {
        log::warn!("Unrecognised RUST_LOG value '{rust_log}', defaulting to {DEFAULT_LEVEL}");
    }

    Ok(())
}

fn log_config() -> Config {
    ConfigBuilder::new()
        .set_time_level(LevelFilter::Error)
        .set_max_level(LevelFilter::Error)
        .set_target_level(LevelFilter::Error)
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .build()
}

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

fn parse_level(raw: &str) -> Option<LevelFilter> {
    raw.split(',')
        .filter_map(|directive| {
            directive
                .split('=')
                .next_back()?
                .parse::<LevelFilter>()
                .ok()
        })
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_level_rejects_empty() {
        assert_eq!(parse_level(""), None);
    }

    #[test]
    fn parse_level_accepts_bare_level_names() {
        assert_eq!(parse_level("error"), Some(LevelFilter::Error));
        assert_eq!(parse_level("warn"), Some(LevelFilter::Warn));
        assert_eq!(parse_level("info"), Some(LevelFilter::Info));
        assert_eq!(parse_level("debug"), Some(LevelFilter::Debug));
        assert_eq!(parse_level("trace"), Some(LevelFilter::Trace));
    }

    #[test]
    fn parse_level_uses_level_of_module_directive() {
        assert_eq!(
            parse_level("backtesting_engine=debug"),
            Some(LevelFilter::Debug)
        );
    }

    #[test]
    fn parse_level_picks_most_verbose_directive() {
        assert_eq!(parse_level("debug,postgres=warn"), Some(LevelFilter::Debug));
        assert_eq!(
            parse_level("info,backtesting_engine=debug"),
            Some(LevelFilter::Debug)
        );
    }

    #[test]
    fn parse_level_case_insensitive() {
        assert_eq!(parse_level("INFO"), Some(LevelFilter::Info));
        assert_eq!(parse_level("Debug"), Some(LevelFilter::Debug));
    }

    #[test]
    fn parse_level_rejects_garbage() {
        assert_eq!(parse_level("nonsense"), None);
    }

    #[test]
    fn parse_level_accepts_off() {
        assert_eq!(parse_level("off"), Some(LevelFilter::Off));
    }

    #[test]
    fn parse_level_rejects_module_without_level() {
        assert_eq!(parse_level("backtesting_engine"), None);
    }

    #[test]
    fn open_log_file_reports_path_when_parent_is_a_file() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        let blocker = dir.path().join("logs");
        std::fs::write(&blocker, "").expect("blocker file creation failed");
        let log_path = blocker.join("bte.log");

        let error = open_log_file(&log_path).expect_err("parent is a file, open must fail");

        let message = format!("{error:#}");
        assert!(
            message.contains(&blocker.display().to_string()),
            "error should name the log directory, got: {message}"
        );
    }

    #[test]
    fn open_log_file_creates_missing_parent_directory() {
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        let log_path = dir.path().join("logs").join("bte.log");

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
        let log_path = dir.path().join("bte.log");

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
        // The global logger is only ever initialized here, so this is safe
        // under both per-process nextest and single-process `cargo test`.
        const MARKER: &str = "init-smoke-marker-9f17";
        let dir = tempfile::tempdir().expect("tempdir creation failed");
        let log_path = dir.path().join("logs").join("bte.log");

        init(None, &log_path).expect("init should create the missing log directory");
        log::error!("{MARKER}");

        let written = std::fs::read_to_string(&log_path).expect("log file readable");
        let line = written
            .lines()
            .rev()
            .find(|l| l.contains(MARKER))
            .expect("marker line should have been written");
        assert_has_dated_timestamp(line);
    }

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
