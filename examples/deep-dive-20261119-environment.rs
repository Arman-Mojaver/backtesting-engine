//! Deep Dive: Environment, DbConfig and parquet_root_dir
//! Source: Code input — src/config/environment.rs
//!
//! One `ENVIRONMENT` variable picks which local PostgreSQL container and which
//! Parquet subtree the engine touches. It is read once at the command entry
//! point; everything downstream receives a `DbConfig` or a `PathBuf`, never the
//! raw variable, so a typo fails fast instead of silently hitting another DB.

use std::env;
use std::ffi::OsStr;

use backtesting_engine::config::environment::{
    DbConfig, Environment, parquet_root_dir, resolve_environment,
};

const ALL: [Environment; 3] = [
    Environment::Production,
    Environment::Development,
    Environment::Testing,
];

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_parsing(),
        "2" => section_2_rejections(),
        "3" => section_3_resolution(),
        "4" => section_4_db_config(),
        "5" => section_5_parquet_root(),
        "6" => section_6_entry_point(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-environment <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Parsing: as_str round-trip and case-insensitive FromStr");
    println!("  2    - Rejections: typos, empty and whitespace values");
    println!("  3    - Resolution: unset, set and non-UTF-8 ENVIRONMENT");
    println!("  4    - DbConfig: per-environment port/database and redacted Debug");
    println!("  5    - Parquet Root: per-environment storage directory");
    println!("  6    - Entry Point: how a CLI command wires it all together");
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

// ============================================================
// SECTION 1: Parsing - as_str Round-Trip and Case-Insensitive FromStr
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_parsing() {
    banner("SECTION 1: Parsing - as_str Round-Trip and Case-Insensitive FromStr");
    // as_str is the canonical lowercase name; it is also the Parquet dir name.
    for env in ALL {
        let round_trip = env.as_str().parse::<Environment>();
        let name = format!("{env:?}");
        println!("{name:<11}.as_str() = {:<11} -> parse -> {round_trip:?}", env.as_str());
    }
    println!();
    // from_str lowercases first, so shell exports in any case work.
    for input in ["PRODUCTION", "Testing", "dEvElOpMeNt"] {
        println!("{input:>13}.parse() -> {:?}", input.parse::<Environment>());
    }
}

// ============================================================
// SECTION 2: Rejections - Typos, Empty and Whitespace Values
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_rejections() {
    banner("SECTION 2: Rejections - Typos, Empty and Whitespace Values");
    // No trimming and no fuzzy matching: anything not exact (modulo case) is an
    // error, because silently choosing a DB is worse than refusing to start.
    for input in ["produciton", "", " production", "production\n", "prod"] {
        let accepted = input.parse::<Environment>().is_ok();
        println!("{:<16} -> accepted: {accepted}", format!("{input:?}"));
    }
    println!();
    println!("The error message names every accepted value:");
    if let Err(err) = "produciton".parse::<Environment>() {
        println!("  {err}");
    }
}

// ============================================================
// SECTION 3: Resolution - Unset, Set and Non-UTF-8 ENVIRONMENT
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_resolution() {
    banner("SECTION 3: Resolution - Unset, Set and Non-UTF-8 ENVIRONMENT");
    // resolve_environment() delegates to the private resolve_from_raw(), which
    // matches on the three shapes env::var can return. The crate forbids
    // `unsafe`, so set_var is off-limits: re-run this section from your shell
    // with different ENVIRONMENT values to drive each match arm.
    let raw = env::var_os("ENVIRONMENT");
    let shown = raw.as_deref().map_or_else(|| "<unset>".into(), OsStr::to_string_lossy);
    println!("ENVIRONMENT in this process: {shown}");
    match resolve_environment() {
        Ok(env) => println!("resolve_environment() -> Ok({env:?})"),
        Err(err) => println!("resolve_environment() -> Err:\n  {err}"),
    }
    let arm = match &raw {
        None => "Err(NotPresent) -> default Development (debug! log, no logger here)",
        Some(v) if v.to_str().is_none() => "Err(NotUnicode) -> fail fast, lossy copy in error",
        Some(_) => "Ok(value) -> FromStr; a bad value fails, never defaults",
    };
    println!("Arm taken: {arm}");
    println!();
    println!("Try each arm:");
    let cmd = "cargo run -q --example deep-dive-20261119-environment 3";
    println!("  env -u ENVIRONMENT {cmd}");
    println!("  ENVIRONMENT=Testing {cmd}");
    println!("  ENVIRONMENT=staging {cmd}");
    println!("  ENVIRONMENT=$'prod\\xffuction' {cmd}");
    println!("  (a lone 0xFF byte is never valid UTF-8, so to_str() rejects it)");
}

// ============================================================
// SECTION 4: DbConfig - Per-Environment Port/Database and Redacted Debug
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_db_config() {
    banner("SECTION 4: DbConfig - Per-Environment Port/Database and Redacted Debug");
    // Only port and database vary: each environment is its own local container.
    // Host and credentials are hard-coded on purpose (local-only engine).
    for env in ALL {
        let c = DbConfig::from_env(env);
        let name = format!("{env:?}");
        println!("{name:<11} -> {}:{} / {} (user={})", c.host, c.port, c.database, c.user);
    }
    println!();
    // Debug is hand-written so `{:?}` in logs/errors never leaks the password,
    // even though the field itself is pub and holds the real value.
    let c = DbConfig::from_env(Environment::Development);
    println!("c.password field = '{}'", c.password);
    println!("Debug output of c:");
    println!("  {c:?}");
}

// ============================================================
// SECTION 5: Parquet Root - Per-Environment Storage Directory
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_parquet_root() {
    banner("SECTION 5: Parquet Root - Per-Environment Storage Directory");
    // The root is the container path joined with as_str(), so test runs can
    // never overwrite production Parquet partitions.
    for env in ALL {
        let name = format!("{env:?}");
        println!("{name:<11} -> {}", parquet_root_dir(env).display());
    }
    println!();
    println!("Nothing is created here: it is a pure path computation.");
    println!("StrategyPersistence::new(root) is what later reads/writes under it.");
}

// ============================================================
// SECTION 6: Entry Point - How a CLI Command Wires It All Together
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_entry_point() {
    banner("SECTION 6: Entry Point - How a CLI Command Wires It All Together");
    println!("CLI detected — run this yourself (touches a real DB and Parquet):");
    println!();
    println!("  ENVIRONMENT=testing bte strategy create \\");
    println!("    --money_management_strategy_id 1 --instrument EURUSD");
    println!();
    println!("src/engine/strategy/create.rs, create_strategies():");
    println!("  1. let env = resolve_environment()?;     // only place env is read");
    println!("  2. DbConfig::from_env(env)               // -> host:54321/db-testing");
    println!("  3. parquet_root_dir(env)                 // -> /app/parquet_files/testing");
    println!("  4. fetch_source_data(&db_config, ..)     // downstream sees DbConfig only");
    println!();
    println!("create_loop.rs follows the same 3-line prologue.");
    println!("With ENVIRONMENT=staging the `?` in step 1 aborts before any I/O.");
}
