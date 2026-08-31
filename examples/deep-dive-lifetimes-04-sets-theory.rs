//! Deep Dive: Lifetimes — Theory of Sets Over Memory
//! Purpose: Understand lifetimes as SETS of program points (regions),
//!          subtyping ('long <: 'short), and lifetime coercion.
//!
//! This is the conceptual model from the Rustonomicon.
//!
//! Run: cargo run --example deep-dive-lifetimes-04-sets-theory
//!      cargo run --example deep-dive-lifetimes-04-sets-theory 1

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_lifetimes_as_regions(),
        "2" => section_2_subtyping_long_is_subtype_of_short(),
        "3" => section_3_lifetime_coercion_in_action(),
        "4" => section_4_set_intuition_summary(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-lifetimes-04-sets-theory <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Lifetimes as Regions: Sets of program points");
    println!("  2    - Subtyping: 'long <: 'short (longer is subtype)");
    println!("  3    - Lifetime Coercion: 'static -> 'a in action");
    println!("  4    - Set Intuition: Summary table");
}

// ============================================================
// SECTION 1: Lifetimes as Regions - Sets of program points
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_lifetimes_as_regions() {
    println!("============================================================");
    println!("SECTION 1: Lifetimes as Regions - Sets of program points");
    println!("============================================================");
    println!();
    println!("A lifetime 'a = a SET of program points where data is valid.");
    println!();
    println!("Example: two variables in nested scopes");
    println!();
    println!("  Program point:  1    2    3    4    5    6    7    8");
    println!("                  |    |    |    |    |    |    |    |");
    println!("  'b (x's life):  |    | [==========] |    |    |    |");
    println!("                  |    | x born   x die|    |    |    |");
    println!("  'a (r's life):  | [========================] |    |");
    println!("                  | r born              r used  |    |");
    println!();
    println!("  As SETS:");
    println!("    'b = {{ 3, 4, 5 }}           (x is alive at these points)");
    println!("    'a = {{ 2,3,4,5,6,7 }}       (r is alive/used at these points)");
    println!();
    println!("  Borrow check rule: data set ⊇ ref set");
    println!("    Is 'b ⊇ 'a?  NO  ->  r = &x would be REJECTED");
    println!("    (x dies before r is used)");
    println!();
    println!("Now let's see a VALID case:");

    // Valid: data lives longer than the reference
    let x = 42; // x born — lives to end of scope
    let r = &x; // r borrows x
    println!("  x = {}, r = {}", x, r);
    println!();
    println!("  'x = {{ 1,2,3,4,5,6,7,8 }}  (x alive for whole scope)");
    println!("  'r = {{ 2,3,4,5 }}           (r alive for subset)");
    println!("  Is 'x ⊇ 'r?  YES  ->  OK!");
}

// ============================================================
// SECTION 2: Subtyping - 'long <: 'short
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_subtyping_long_is_subtype_of_short() {
    println!("============================================================");
    println!("SECTION 2: Subtyping: 'long <: 'short");
    println!("============================================================");
    println!();
    println!("SUBTYPING RULE:");
    println!("  'long <: 'short  iff  'long completely CONTAINS 'short");
    println!();
    println!("  'long:  [================================]  (bigger region)");
    println!("  'short:       [==================]          (smaller region)");
    println!();
    println!("  'long's region ⊇ 'short's region");
    println!("  Therefore 'long is a SUBTYPE of 'short");
    println!();
    println!("  Intuition: 'If it lives long enough,");
    println!("             it certainly lives long enough for this shorter use.'");
    println!();
    println!("Consequence: &'static str can be used where &'a str is needed");

    // debug expects two &'a str (same lifetime).
    // hello is &'static, world is &'world (shorter).
    // hello is silently coerced from &'static to &'world.
    let hello: &'static str = "hello";
    {
        let world = String::from("world");
        let world_ref = &world; // 'world — shorter lifetime
        debug(hello, world_ref); // hello coerced: &'static -> &'world
    }
    println!();
    println!("  debug<'a>(a: &'a str, b: &'a str)");
    println!("  -> hello (&'static) coerced to &'world");
    println!("  -> Legal because 'static <: 'world (static outlives everything)");
    println!("  -> The coercion is SILENT and AUTOMATIC");
}

// ============================================================
// SECTION 3: Lifetime Coercion - 'static -> 'a in action
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_lifetime_coercion_in_action() {
    println!("============================================================");
    println!("SECTION 3: Lifetime Coercion: 'static -> 'a in action");
    println!("============================================================");
    println!();
    println!("Coercion = converting a LONGER lifetime into a SHORTER one.");
    println!("Only goes ONE way: longer -> shorter (never shorter -> longer)");
    println!();

    // Demonstrating coercion explicitly
    let static_str: &'static str = "I live forever";
    let coerced = coerce_to_shorter(static_str);
    println!("Coerced: \"{}\"", coerced);
    println!();
    println!("  fn coerce_to_shorter<'a>(s: &'a str) -> &'a str {{ s }}");
    println!("  -> Called with &'static str");
    println!("  -> 'a becomes 'static (the actual lifetime)");
    println!("  -> &'static str satisfies &'a str because 'static <: 'a");
    println!();

    // What would NOT work (shorter -> longer):
    println!("What would FAIL (shorter -> longer):");
    println!("  fn make_static<'a>(s: &'a str) -> &'static str {{ s }}");
    println!("  // ERROR: cannot coerce 'a to 'static");
    println!("  // 'a might be shorter than 'static -> would be dangling");
    println!();
    println!("  Coercion direction:");
    println!("    'static -> 'a   OK   (restrict to subset)");
    println!("    'a -> 'static   FAIL (extend to superset = dangling!)");
}

// ============================================================
// SECTION 4: Set Intuition - Summary table
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_set_intuition_summary() {
    println!("============================================================");
    println!("SECTION 4: Set Intuition - Summary table");
    println!("============================================================");
    println!();
    println!("| Concept           | Set Theory View    | Practical Meaning              |");
    println!("|-------------------|--------------------|-------------------------------|");
    println!("| Lifetime 'a       | Set of prog points | 'Valid in this region'        |");
    println!("| 'a: 'b (outlives) | 'a >= 'b (superset)| ''a's region contains 'b's'   |");
    println!("| 'long <: 'short   | 'long >= 'short    | 'Longer is subtype of shorter'|");
    println!("| Coercion          | Restrict to subset | 'Use long ref for short use'  |");
    println!("| Borrow check      | data >= ref        | 'Data set contains ref set'   |");
    println!();
    println!("The 99% intuition (from the Rustonomicon):");
    println!("  'long <: 'short  iff  'long completely contains 'short");
    println!("  'long may be larger, but that's fine.");
    println!();
    println!("  Unless you write unsafe code, the compiler handles");
    println!("  all corner cases automatically.");
    println!();
    println!("Key takeaway: Think in REGIONS (sets of program points),");
    println!("  not in 'annotations on function signatures.'");
    println!("  The annotations are just labels for these regions.");
}

// ---- Helper functions ----

/// Demonstrates that &'static can be used where &'a is expected.
fn debug<'a>(a: &'a str, b: &'a str) {
    println!("  debug(): a=\"{}\", b=\"{}\"", a, b);
}

/// Shows coercion: accepts &'a str, called with &'static str.
fn coerce_to_shorter<'a>(s: &'a str) -> &'a str {
    s
}
