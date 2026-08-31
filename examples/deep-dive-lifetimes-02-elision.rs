//! Deep Dive: Lifetimes — Elision Rules
//! Purpose: See the 3 lifetime elision rules the compiler applies automatically.
//!          These are the lifetimes that exist by default without you writing them.
//!
//! Run: cargo run --example deep-dive-lifetimes-02-elision
//!      cargo run --example deep-dive-lifetimes-02-elision 1

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_rule1_each_input_own_lifetime(),
        "2" => section_2_rule2_single_input_to_output(),
        "3" => section_3_rule3_self_to_output(),
        "4" => section_4_when_elision_fails(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-lifetimes-02-elision <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Rule 1: Each Input Gets Its Own Lifetime");
    println!("  2    - Rule 2: Single Input Lifetime -> All Outputs");
    println!("  3    - Rule 3: Method &self Lifetime -> All Outputs");
    println!("  4    - When Elision Fails: You Must Be Explicit");
}

// ============================================================
// SECTION 1: Rule 1 - Each Input Gets Its Own Lifetime
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_rule1_each_input_own_lifetime() {
    println!("============================================================");
    println!("SECTION 1: Rule 1 - Each Input Gets Its Own Lifetime");
    println!("============================================================");
    println!();
    println!("Rule 1: Every &input without an explicit lifetime");
    println!("        gets a FRESH, UNIQUE lifetime.");
    println!();
    println!("You write:   fn foo(x: &str, y: &str)");
    println!("Compiler:    fn foo<'a, 'b>(x: &'a str, y: &'b str)");
    println!();
    println!("Each & gets its OWN lifetime — they are NOT the same.");
    println!();

    // This function has two inputs with two independent lifetimes.
    // No output reference, so Rules 2 and 3 don't apply.
    let s1 = String::from("hello");
    let s2 = String::from("world");
    print_two(&s1, &s2);
    println!();
    println!("  -> s1 has lifetime 'a, s2 has lifetime 'b");
    println!("  -> They are independent — no relationship assumed");
}

// ============================================================
// SECTION 2: Rule 2 - Single Input Lifetime -> All Outputs
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_rule2_single_input_to_output() {
    println!("============================================================");
    println!("SECTION 2: Rule 2 - Single Input Lifetime -> All Outputs");
    println!("============================================================");
    println!();
    println!("Rule 2: If there's exactly ONE input lifetime,");
    println!("        it's assigned to ALL output lifetimes.");
    println!();
    println!("You write:   fn first_word(s: &str) -> &str");
    println!("Step 1:      fn first_word<'a>(s: &'a str) -> &str   (Rule 1)");
    println!("Step 2:      fn first_word<'a>(s: &'a str) -> &'a str (Rule 2)");
    println!();

    let sentence = String::from("the quick brown fox");
    let word = first_word(&sentence);
    println!("first_word(\"{}\") = \"{}\"", sentence, word);
    println!();
    println!("  -> One input -> output borrows from it");
    println!("  -> The output CANNOT outlive the input");
    println!("  -> This is always correct: where else would it borrow from?");
}

// ============================================================
// SECTION 3: Rule 3 - Method &self Lifetime -> All Outputs
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_rule3_self_to_output() {
    println!("============================================================");
    println!("SECTION 3: Rule 3 - Method &self Lifetime -> All Outputs");
    println!("============================================================");
    println!();
    println!("Rule 3: In methods, if &self or &mut self exists,");
    println!("        self's lifetime goes to ALL output lifetimes.");
    println!();
    println!("You write:   fn part(&self, ann: &str) -> &str");
    println!("Compiler:    fn part<'a,'b>(&'a self, ann: &'b str) -> &'a str");
    println!();

    let novel = String::from("Call me Ishmael. Some years ago...");
    let first = novel.split('.').next().unwrap();
    let excerpt = ImportantExcerpt { part: first };
    let returned = excerpt.announce_and_return_part("Attention!");
    println!("Returned from method: \"{}\"", returned);
    println!();
    println!("  -> Method has &self + another &input (two lifetimes)");
    println!("  -> Rule 2 doesn't apply (more than one input)");
    println!("  -> Rule 3: output gets SELF's lifetime, not ann's");
    println!("  -> Makes sense: methods usually return data from self");
}

// ============================================================
// SECTION 4: When Elision Fails - You Must Be Explicit
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_when_elision_fails() {
    println!("============================================================");
    println!("SECTION 4: When Elision Fails - You Must Be Explicit");
    println!("============================================================");
    println!();
    println!("Elision fails when the compiler can't determine");
    println!("which input the output borrows from.");
    println!();
    println!("Case 1: No inputs to infer from");
    println!("  fn get_str() -> &str;  // ILLEGAL — where does it come from?");
    println!();
    println!("Case 2: Multiple inputs, no &self (ambiguous)");
    println!("  fn frob(s: &str, t: &str) -> &str;  // ILLEGAL — s or t?");
    println!();
    println!("FIX: Write explicit lifetimes:");

    let s1 = String::from("long string here");
    let s2 = String::from("short");
    let result = longest(&s1, &s2);
    println!("  longest(\"{}\", \"{}\") = \"{}\"", s1, s2, result);
    println!();
    println!("  fn longest<'a>(x: &'a str, y: &'a str) -> &'a str");
    println!("  -> 'a ties both inputs and output together");
    println!("  -> Output lives as long as the SHORTER of x and y");
}

// ---- Helper functions demonstrating elision ----

/// Rule 1 demo: two inputs, no output reference.
/// Compiler sees: fn print_two<'a, 'b>(x: &'a str, y: &'b str)
fn print_two(x: &str, y: &str) {
    println!("  print_two: x=\"{}\", y=\"{}\"", x, y);
}

/// Rule 2 demo: one input, one output.
/// Compiler sees: fn first_word<'a>(s: &'a str) -> &'a str
fn first_word(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b' ' {
            return &s[0..i];
        }
    }
    s
}

/// Struct holding a reference — needs a lifetime parameter.
struct ImportantExcerpt<'a> {
    part: &'a str,
}

impl<'a> ImportantExcerpt<'a> {
    /// Rule 3 demo: method with &self + another input.
    /// Compiler sees: fn announce_and_return_part<'b>(&'b self, ann: &'c str) -> &'b str
    fn announce_and_return_part(&self, announcement: &str) -> &str {
        println!("  Announcement: {}", announcement);
        self.part
    }
}

/// Explicit lifetimes needed: two inputs, returns one of them.
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
