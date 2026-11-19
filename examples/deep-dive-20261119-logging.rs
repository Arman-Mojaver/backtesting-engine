//! Deep Dive: logging::init and its RUST_LOG handling
//! Source: Code input — src/infrastructure/logging.rs
//!
//! One `init` call at the binary entry point fans every `log::*!` macro out to
//! stderr (container output) and an append-only `logs/bte.log` (persistence).
//! simplelog has no per-module filtering, so RUST_LOG is collapsed to a single
//! global level. Each section runs in its own process, so each may install the
//! global logger once. All files go to a tempdir; nothing touches `logs/`.

use std::env;
use std::fs;
use std::path::Path;

use backtesting_engine::infrastructure::logging::{self, LOG_FILE};
use log::LevelFilter;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_directives(),
        "2" => section_2_fallback(),
        "3" => section_3_real_init(),
        "4" => section_4_double_init(),
        "5" => section_5_open_errors(),
        "6" => section_6_entry_point(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-logging <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Directives: how RUST_LOG collapses to one global level");
    println!("  2    - Fallback: when the default level and the warning kick in");
    println!("  3    - Real Init: dual sinks, filtering, line format, append mode");
    println!("  4    - Double Init: the global logger can only be set once");
    println!("  5    - Open Errors: blocked log directory and empty parent path");
    println!("  6    - Entry Point: how bte wires RUST_LOG and LOG_FILE");
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

/// Verbatim copy of the private `parse_level` (examples cannot reach private
/// items). If the real one changes, its unit tests are the source of truth.
fn mirror_parse_level(raw: &str) -> Option<LevelFilter> {
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

// ============================================================
// SECTION 1: Directives - How RUST_LOG Collapses to One Global Level
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_directives() {
    banner("SECTION 1: Directives - How RUST_LOG Collapses to One Global Level");
    // LevelFilter is ordered Off < Error < Warn < Info < Debug < Trace,
    // so `.max()` picks the MOST verbose directive.
    for raw in [
        "warn",
        "DEBUG",
        "backtesting_engine=debug",
        "info,backtesting_engine=debug",
        "debug,postgres=warn",
        "off,trace",
        "nonsense,warn",
    ] {
        println!("  {raw:<32} -> {:?}", mirror_parse_level(raw));
    }
    println!();
    println!("  -> Only the text after the LAST '=' matters; the module is dropped.");
    println!("  -> 'debug,postgres=warn' still logs postgres at DEBUG: there is");
    println!("     no per-module filter, the max level applies to every crate.");
    println!("  -> Garbage directives are skipped silently if any other parses.");
}

// ============================================================
// SECTION 2: Fallback - When the Default Level and the Warning Kick In
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_fallback() {
    banner("SECTION 2: Fallback - When the Default Level and the Warning Kick In");
    // Mirrors the decision logic at the top and bottom of `init`.
    for rust_log in [None, Some(""), Some("nonsense"), Some("backtesting_engine"), Some("off")] {
        let raw = rust_log.unwrap_or_default();
        let parsed = mirror_parse_level(raw);
        let level = parsed.unwrap_or(LevelFilter::Info);
        let warns = parsed.is_none() && !raw.is_empty();
        let shown = format!("{rust_log:?}");
        let parsed = format!("{parsed:?}");
        println!("  {shown:<26} parsed={parsed:<10} level={level:<5} warn={warns}");
    }
    println!();
    println!("  -> Unset and empty RUST_LOG fall back to INFO quietly.");
    println!("  -> A bare module name has no level, so it warns like garbage.");
    println!("  -> 'off' is a real level: it parses and silences everything,");
    println!("     including the warning (which is emitted AFTER install).");
}

// ============================================================
// SECTION 3: Real Init - Dual Sinks, Filtering, Line Format, Append Mode
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_real_init() {
    banner("SECTION 3: Real Init - Dual Sinks, Filtering, Line Format, Append Mode");
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("logs").join("bte.log");
    println!("logs/ exists before init: {}", log_path.parent().unwrap().exists());

    // Simulates a previous run so append mode is visible.
    fs::create_dir_all(log_path.parent().unwrap()).expect("mkdir");
    fs::write(&log_path, "<line from a previous run>\n").expect("seed");

    println!("--- stderr sink (TermLogger) follows ---");
    logging::init(Some("deep-dive-typo"), &log_path).expect("init");
    log::info!("info passes the INFO fallback level");
    log::debug!("debug is filtered out at INFO");
    log::error!("error passes too");
    println!("--- file sink (WriteLogger) contents ---");
    for line in fs::read_to_string(&log_path).expect("read").lines() {
        println!("  {line}");
    }
    println!();
    println!("  -> Previous content kept: the file opens in append mode.");
    println!("  -> The debug! line is in neither sink; both share one level.");
}

// ============================================================
// SECTION 4: Double Init - The Global Logger Can Only Be Set Once
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_double_init() {
    banner("SECTION 4: Double Init - The Global Logger Can Only Be Set Once");
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("first.log");
    let second = dir.path().join("second.log");

    println!("first init:  {:?}", logging::init(Some("warn"), &first).map_err(|e| format!("{e:#}")));
    println!("second init: {:?}", logging::init(Some("warn"), &second).map_err(|e| format!("{e:#}")));
    println!("second.log created anyway: {}", second.exists());
    println!();
    println!("  -> log::set_logger is process-global and one-shot.");
    println!("  -> The file is opened BEFORE install, so a failed second init");
    println!("     still leaves an empty file behind.");
    println!("  -> This is why the unit test suite calls init exactly once.");
}

// ============================================================
// SECTION 5: Open Errors - Blocked Log Directory and Empty Parent Path
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_open_errors() {
    banner("SECTION 5: Open Errors - Blocked Log Directory and Empty Parent Path");
    let dir = tempfile::tempdir().expect("tempdir");
    // A regular FILE named `logs` where the directory should be.
    let blocker = dir.path().join("logs");
    fs::write(&blocker, "").expect("blocker");
    let result = logging::init(None, &blocker.join("bte.log"));
    let message = format!("{:#}", result.expect_err("must fail"));
    println!("init error chain (`{{:#}}`):");
    for part in message.split(": ") {
        println!("  -> {part}");
    }
    println!();
    // A bare filename has parent Some(""), not None; create_dir_all("") is Ok.
    let bare = Path::new("bte.log");
    println!("Path::new(\"bte.log\").parent() = {:?}", bare.parent());
    println!("fs::create_dir_all(\"\")        = {:?}", fs::create_dir_all(""));
    println!();
    println!("  -> The error names the directory, not just the file.");
    println!("  -> Failure happens before install, so no logger is set and");
    println!("     the binary reports it with eprintln! instead.");
}

// ============================================================
// SECTION 6: Entry Point - How bte Wires RUST_LOG and LOG_FILE
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_entry_point() {
    banner("SECTION 6: Entry Point - How bte Wires RUST_LOG and LOG_FILE");
    println!("CLI detected: these commands are printed, never executed.");
    println!();
    println!("  RUST_LOG=debug cargo run --bin bte -- <command>");
    println!("  RUST_LOG=bogus cargo run --bin bte -- <command>");
    println!("  tail -f {LOG_FILE}");
    println!();
    println!("Flow in src/bin/bte.rs:");
    println!("  1. Parse CLI args (clap) first, so --help works without logging");
    println!("  2. env::var(\"RUST_LOG\").ok() -> Option<String>");
    println!("  3. logging::init(rust_log, Path::new(\"{LOG_FILE}\"))");
    println!("     Err -> eprintln!(\"Error: {{e:#}}\") + exit 1 (no logger yet)");
    println!("  4. run(); Err -> log::error! -> stderr AND {LOG_FILE}");
    println!();
    println!("  -> LOG_FILE is relative: it lands under the CURRENT directory.");
}
