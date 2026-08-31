//! Deep Dive: Lifetimes — Explicit Annotations
//! Purpose: When elision fails, write lifetimes explicitly.
//!          Covers functions, structs, and multiple lifetimes.
//!
//! Run: cargo run --example deep-dive-lifetimes-03-explicit
//!      cargo run --example deep-dive-lifetimes-03-explicit 1

use std::env;
use std::fmt::Display;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_syntax_and_functions(),
        "2" => section_2_structs_holding_references(),
        "3" => section_3_multiple_lifetimes(),
        "4" => section_4_generics_traits_lifetimes_combined(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-lifetimes-03-explicit <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Syntax & Functions: The 'a notation");
    println!("  2    - Structs Holding References: Lifetime on the struct");
    println!("  3    - Multiple Lifetimes: Different lifetimes for different refs");
    println!("  4    - Combined: Generics + Traits + Lifetimes");
}

// ============================================================
// SECTION 1: Syntax & Functions - The 'a notation
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_syntax_and_functions() {
    println!("============================================================");
    println!("SECTION 1: Syntax & Functions - The 'a notation");
    println!("============================================================");
    println!();
    println!("Lifetime syntax:");
    println!("  &i32          — a reference (lifetime elided)");
    println!("  &'a i32       — a reference with explicit lifetime 'a");
    println!("  &'a mut i32   — a mutable reference with lifetime 'a");
    println!();
    println!("Key: annotations DON'T change lifetimes — they describe");
    println!("     RELATIONSHIPS between lifetimes.");
    println!();

    let x = 10;
    let y = 20;
    let max = max_ref(&x, &y);
    println!("max_ref({}, {}) = {}", x, y, max);
    println!();
    println!("  fn max_ref<'a>(x: &'a i32, y: &'a i32) -> &'a i32");
    println!("  -> 'a says: output lives as long as the shorter input");
    println!("  -> Both inputs share 'a, so 'a = min(x_life, y_life)");
}

// ============================================================
// SECTION 2: Structs Holding References
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_structs_holding_references() {
    println!("============================================================");
    println!("SECTION 2: Structs Holding References");
    println!("============================================================");
    println!();
    println!("A struct that BORROWS (doesn't own) needs a lifetime:");
    println!("  struct ImportantExcerpt<'a> {{ part: &'a str }}");
    println!();
    println!("The lifetime says: struct can't outlive the borrowed data.");
    println!();

    let novel = String::from("Call me Ishmael. Some years ago...");
    let first_sentence = novel.split('.').next().unwrap();

    // excerpt borrows from novel. If novel drops first -> dangling.
    // The lifetime 'a prevents this at compile time.
    let excerpt = ImportantExcerpt {
        part: first_sentence,
    };
    println!("Excerpt: \"{}\"", excerpt.part);
    println!();
    println!("  -> 'a ties the struct to novel's lifetime");
    println!("  -> excerpt cannot outlive novel");
    println!("  -> Compiler verifies this at every use");
}

// ============================================================
// SECTION 3: Multiple Lifetimes
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_multiple_lifetimes() {
    println!("============================================================");
    println!("SECTION 3: Multiple Lifetimes");
    println!("============================================================");
    println!();
    println!("Sometimes different references need DIFFERENT lifetimes:");
    println!();

    let config = String::from("config_value");
    let input = String::from("input_data");

    let ctx = Context {
        config: &config,
        input: &input,
    };
    println!(
        "Context: config=\"{}\", input=\"{}\"",
        ctx.config, ctx.input
    );
    println!();

    let parsed = parse(&config, &input);
    println!("parse() returned: \"{}\"", parsed);
    println!();
    println!("  struct Context<'a, 'b> {{ config: &'a str, input: &'b str }}");
    println!("  -> 'a and 'b are INDEPENDENT");
    println!("  -> config and input can have different lifetimes");
    println!();
    println!("  fn parse<'a, 'b>(config: &'a str, input: &'b str) -> &'a str");
    println!("  -> Output tied to 'a (config) only, NOT 'b (input)");
    println!("  -> Caller knows: result lives as long as config, not input");
}

// ============================================================
// SECTION 4: Combined - Generics + Traits + Lifetimes
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_generics_traits_lifetimes_combined() {
    println!("============================================================");
    println!("SECTION 4: Combined - Generics + Traits + Lifetimes");
    println!("============================================================");
    println!();
    println!("Lifetimes, generics, and trait bounds in one signature:");
    println!();

    let x = "very long string";
    let y = "short";
    let announcement = "Comparing strings!";
    let result = longest_with_announcement(x, y, announcement);
    println!("Result: \"{}\"", result);
    println!();
    println!("  fn longest_with_announcement<'a, T>(");
    println!("      x: &'a str, y: &'a str, ann: T,");
    println!("  ) -> &'a str");
    println!("  where T: Display");
    println!();
    println!("  -> 'a: lifetime parameter (ties x, y, and output)");
    println!("  -> T:  generic type parameter");
    println!("  -> T: Display: trait bound (T must be printable)");
    println!("  -> All three work together in one signature");
}

// ---- Helper functions and structs ----

/// Function with explicit lifetime — returns the larger reference.
fn max_ref<'a>(x: &'a i32, y: &'a i32) -> &'a i32 {
    if x >= y {
        x
    } else {
        y
    }
}

/// Struct that borrows a string slice — needs lifetime parameter.
struct ImportantExcerpt<'a> {
    part: &'a str,
}

/// Struct with TWO independent lifetimes.
struct Context<'a, 'b> {
    config: &'a str,
    input: &'b str,
}

/// Function returning a reference tied to only ONE input.
fn parse<'a, 'b>(config: &'a str, _input: &'b str) -> &'a str {
    // Returns something borrowing from config, not input.
    // Output lifetime is 'a, NOT 'b.
    config
}

/// Combining lifetimes, generics, and trait bounds.
fn longest_with_announcement<'a, T>(x: &'a str, y: &'a str, ann: T) -> &'a str
where
    T: Display,
{
    println!("  Announcement: {}", ann);
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
