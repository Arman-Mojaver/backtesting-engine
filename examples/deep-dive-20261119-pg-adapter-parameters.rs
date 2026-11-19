//! Deep Dive: pg_adapter (1/2) — turning `indicator.parameters` JSON into IndicatorParameters
//! Source: Code input — src/infrastructure/pg_adapter.rs
//!
//! The `indicator` table is owned by the Django companion: `type` is a short
//! string and `parameters` is free-form JSON. map_parameters is the single gate
//! that turns that untyped pair into the typed IndicatorParameters enum, and it
//! is deliberately strict — a bad row fails the load instead of silently
//! producing a wrong indicator.
//!
//! map_parameters is private, and no Row can be built without a live database.
//! So the real source file is pulled in verbatim with include! inside the
//! `pg` module below; the shims re-export the library types under the
//! `crate::...` paths the file expects. Nothing in src/ is modified.

use std::env;

mod config {
    pub mod environment {
        pub use backtesting_engine::config::environment::DbConfig;
    }
}
mod domain {
    pub mod indicator {
        pub use backtesting_engine::domain::indicator::*;
    }
    pub mod operation_point {
        pub use backtesting_engine::domain::operation_point::*;
    }
    pub mod resampled_point {
        pub use backtesting_engine::domain::resampled_point::*;
    }
}

#[allow(dead_code)]
mod pg {
    include!("../src/infrastructure/pg_adapter.rs");

    // Same module as the included items, so the private fn is reachable.
    pub fn map(indicator_type: &str, json_text: &str) -> anyhow::Result<IndicatorParameters> {
        map_parameters(indicator_type, json_text)
    }
}

fn show(indicator_type: &str, json_text: &str) {
    match pg::map(indicator_type, json_text) {
        Ok(p) => println!("  {indicator_type:<6} {json_text:<44} -> {p:?}"),
        Err(e) => println!("  {indicator_type:<6} {json_text:<44} -> Err: {}", e.root_cause()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_db_json_shapes(),
        "2" => section_2_parameterless_types(),
        "3" => section_3_serde_strictness(),
        "4" => section_4_error_context_chain(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-pg-adapter-parameters <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - DB JSON Shapes: one Params struct per indicator family");
    println!("  2    - Parameterless Types: ao/ac accept only an empty object");
    println!("  3    - Serde Strictness: what deny_unknown_fields and i32 reject");
    println!("  4    - Error Context Chain: which row failed, and why");
}

// ============================================================
// SECTION 1: DB JSON Shapes - one Params struct per indicator family
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_db_json_shapes() {
    println!("============================================================");
    println!("SECTION 1: DB JSON Shapes - one Params struct per indicator family");
    println!("============================================================");
    // Literals mirror rows captured from the dev DB: one space after each colon.
    show("macd", r#"{"slow_type": "sma", "slow_n": 20, "fast_type": "ema", "fast_n": 21}"#);
    show("rsi", r#"{"n": 10, "high_threshold": 80, "low_threshold": 20}"#);
    show("cho", r#"{"fast_ma": 30, "slow_ma": 40}"#);
    show("osma", r#"{"fast_ema": 12, "slow_ema": 26, "signal_sma": 9}"#);
    println!();
    // Six type strings share PeriodParams but map to six different variants.
    for t in ["bears", "bulls", "dpo", "force", "roc", "vroc"] {
        show(t, r#"{"period": 105}"#);
    }
    show("roc", r#"{"period":105}"#);
    println!();
    println!("  -> The type string picks the Params struct; the struct fixes the keys");
    println!("  -> PeriodParams is reused: the match arm, not the JSON, picks the variant");
    println!("  -> Whitespace is irrelevant: compact JSON parses the same");
}

// ============================================================
// SECTION 2: Parameterless Types - ao/ac accept only an empty object
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_parameterless_types() {
    println!("============================================================");
    println!("SECTION 2: Parameterless Types - ao/ac accept only an empty object");
    println!("============================================================");
    show("ao", "{}");
    show("ac", "{}");
    println!();
    show("ao", r#"{"n": 14}"#);
    show("ac", r#"{"fast": 5, "slow": 34}"#);
    show("ao", "[]");
    show("ao", "null");
    println!();
    println!("  -> parse_empty_params parses into Map<String, Value>, then checks is_empty()");
    println!("  -> A unit struct with deny_unknown_fields would also work; the Map");
    println!("     route lets the error list every stray key by name");
    println!("  -> `[]` and `null` are not objects, so they fail in serde first");
}

// ============================================================
// SECTION 3: Serde Strictness - what deny_unknown_fields and i32 reject
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_serde_strictness() {
    println!("============================================================");
    println!("SECTION 3: Serde Strictness - what deny_unknown_fields and i32 reject");
    println!("============================================================");
    println!("Missing key:");
    show("rsi", r#"{"n": 10, "high_threshold": 80}"#);
    println!("Extra key (deny_unknown_fields):");
    show("bears", r#"{"period": 5, "signal": 9}"#);
    println!("Wrong JSON type for an i32:");
    show("roc", r#"{"period": "105"}"#);
    show("roc", r#"{"period": 10.5}"#);
    show("roc", r#"{"period": 3000000000}"#);
    println!("Not an object at all:");
    show("vroc", "12");
    println!();
    println!("  -> Every one of these is a typo or a schema drift on the Django side");
    println!("  -> Failing here aborts the load; a silent default would compute a");
    println!("     different indicator than the row describes, under the row's id");
}

// ============================================================
// SECTION 4: Error Context Chain - which row failed, and why
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_error_context_chain() {
    println!("============================================================");
    println!("SECTION 4: Error Context Chain - which row failed, and why");
    println!("============================================================");
    let cases = [
        ("macd", r#"{"slow_type": "sma", "slow_n": 20, "fast_type": "wma", "fast_n": 21}"#),
        ("stochastic", r#"{"n": 1}"#),
        ("rsi", r#"{"n": 10}"#),
    ];
    for (indicator_type, json_text) in cases {
        // row_to_indicator adds this outermost layer with the row id.
        let err = pg::map(indicator_type, json_text)
            .map_err(|e| e.context("failed to map parameters for indicator id 2001"))
            .unwrap_err();
        println!("type \"{indicator_type}\":");
        for (depth, cause) in err.chain().enumerate() {
            println!("  [{depth}] {cause}");
        }
    }
    println!();
    println!("  -> [0] is always the row id, added by row_to_indicator");
    println!("  -> Serde failures get a middle layer from parse_params: type + raw JSON");
    println!("  -> MA-type and unknown-type failures are bare bail!s one level down");
}
