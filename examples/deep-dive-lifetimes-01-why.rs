//! Deep Dive: Lifetimes — The "Why"
//! Purpose: Understand WHY lifetimes exist by seeing dangling references
//!          and the borrow checker in action.
//!
//! Run: cargo run --example deep-dive-lifetimes-01-why
//!      cargo run --example deep-dive-lifetimes-01-why 1

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_valid_reference(),
        "2" => section_2_dangling_reference_explained(),
        "3" => section_3_borrow_checker_rules(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-lifetimes-01-why <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Valid Reference: A reference that works fine");
    println!("  2    - Dangling Reference: Why it fails (explained, not executed)");
    println!("  3    - Borrow Checker Rules: The 3 rules it enforces");
}

// ============================================================
// SECTION 1: Valid Reference - A reference that works fine
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_valid_reference() {
    println!("============================================================");
    println!("SECTION 1: Valid Reference - A reference that works fine");
    println!("============================================================");
    println!();

    // Here the data (x) lives LONGER than the reference (r).
    // The reference's lifetime is contained within the data's lifetime.
    let x = 5; // x created here — lives until end of main
    let r = &x; // r borrows x — r's lifetime starts here
    println!("x = {}, r = {}", x, r);
    // r is used here while x is still alive. OK!
    // Both x and r are dropped at end of scope — no dangling.

    println!();
    println!("  -> x lives until end of scope");
    println!("  -> r borrows x and is used while x is alive");
    println!("  -> Borrow checker: data lifetime CONTAINS ref lifetime -> OK");
}

// ============================================================
// SECTION 2: Dangling Reference - Why it fails
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_dangling_reference_explained() {
    println!("============================================================");
    println!("SECTION 2: Dangling Reference - Why it fails (explained)");
    println!("============================================================");
    println!();
    println!("This code would create a dangling reference:");
    println!();
    println!("  fn main() {{");
    println!("      let r;                // r declared (lifetime 'a starts)");
    println!("      {{");
    println!("          let x = 5;        // x declared (lifetime 'b starts)");
    println!("          r = &x;           // r borrows x");
    println!("      }}                    // x dropped here (lifetime 'b ends)");
    println!("      println!(\"{{}}\", r);  // r used here — but x is gone!");
    println!("  }}");
    println!();
    println!("Compiler error (E0597):");
    println!("  error: `x` does not live long enough");
    println!("    -> x dropped at line 5 while still borrowed");
    println!("    -> borrow used at line 6");
    println!();
    println!("As SETS of program points:");
    println!("  'b (x's life)   = {{ 3, 4 }}        (smaller set)");
    println!("  'a (r's life)   = {{ 2,3,4,5,6 }}   (larger set)");
    println!();
    println!("  Is 'b ⊇ 'a? NO — x's set does NOT contain r's set.");
    println!("  The reference outlives the data -> REJECTED.");
    println!();
    println!("FIX: Move x to the outer scope so it lives as long as r.");
}

// ============================================================
// SECTION 3: Borrow Checker Rules - The 3 rules it enforces
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_borrow_checker_rules() {
    println!("============================================================");
    println!("SECTION 3: Borrow Checker Rules - The 3 rules it enforces");
    println!("============================================================");
    println!();
    println!("The borrow checker enforces 3 rules:");
    println!();
    println!("  1. Each reference has a lifetime (region of validity)");
    println!("  2. Each data has a lifetime (region of existence)");
    println!("  3. ref lifetime ⊆ data lifetime (ref must not outlive data)");
    println!();
    println!("Demonstrating rule 3 with a function returning a reference:");

    let string = String::from("hello world");
    let word = first_word(&string);
    println!("  first_word(\"{}\") = \"{}\"", string, word);
    println!();
    println!("  -> string lives until end of scope");
    println!("  -> word borrows from string (same lifetime)");
    println!("  -> word is used while string is alive -> OK");

    println!();
    println!("What if we tried to use 'word' after 'string' is dropped?");
    println!("  let word;");
    println!("  {{");
    println!("      let string = String::from(\"hello\");");
    println!("      word = first_word(&string);");
    println!("  }}  // string dropped here");
    println!("  println!(\"{{}}\", word);  // ERROR: string doesn't live long enough");
    println!();
    println!("  -> The borrow checker catches this at COMPILE TIME.");
    println!("  -> Zero runtime cost — no garbage collector needed.");
}

/// Returns the first word of a string slice.
/// The returned &str borrows from the input — same lifetime.
/// (Elision Rule 2: one input lifetime -> output gets it too)
fn first_word(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b' ' {
            return &s[0..i];
        }
    }
    s
}
