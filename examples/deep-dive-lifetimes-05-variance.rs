//! Deep Dive: Lifetimes — Variance & Subtyping
//! Purpose: Understand covariance, contravariance, and invariance,
//!          and why &mut T is invariant (prevents use-after-free).
//!
//! Run: cargo run --example deep-dive-lifetimes-05-variance
//!      cargo run --example deep-dive-lifetimes-05-variance 1

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_three_kinds_of_variance(),
        "2" => section_2_variance_table(),
        "3" => section_3_why_mut_is_invariant(),
        "4" => section_4_contravariance_of_function_args(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-lifetimes-05-variance <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Three Kinds of Variance: covariant, contravariant, invariant");
    println!("  2    - Variance Table: How common types behave");
    println!("  3    - Why &mut T Is Invariant: Preventing use-after-free");
    println!("  4    - Contravariance of Function Arguments");
}

// ============================================================
// SECTION 1: Three Kinds of Variance
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_three_kinds_of_variance() {
    println!("============================================================");
    println!("SECTION 1: Three Kinds of Variance");
    println!("============================================================");
    println!();
    println!("Given Sub <: Super, how does F<Sub> relate to F<Super>?");
    println!();
    println!("  COVARIANT:     F<Sub> <: F<Super>");
    println!("                Subtyping passes through unchanged");
    println!();
    println!("  CONTRAVARIANT: F<Super> <: F<Sub>");
    println!("                Subtyping is INVERTED");
    println!();
    println!("  INVARIANT:     No relationship at all");
    println!("                F<Sub> and F<Super> must match EXACTLY");
    println!();
    println!("For lifetimes: Sub = 'long, Super = 'short");
    println!("  ('long <: 'short because 'long's region contains 'short's)");
    println!();

    // Covariance demo: &'a T is covariant over 'a
    let static_str: &'static str = "static";
    let local = String::from("local");
    let local_ref: &str = &local;

    // Both can be passed to a function expecting &'a str
    print_ref(static_str); // &'static -> &'a (covariant, OK)
    print_ref(local_ref); // &'local -> &'a (exact match, OK)
    println!();
    println!("  &'a T is COVARIANT over 'a");
    println!("  -> &'static str can be used where &'a str is expected");
    println!("  -> Because 'static <: 'a, and covariance passes it through");
}

// ============================================================
// SECTION 2: Variance Table
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_variance_table() {
    println!("============================================================");
    println!("SECTION 2: Variance Table");
    println!("============================================================");
    println!();
    println!("| Type            | over 'a       | over T             |");
    println!("|-----------------|---------------|--------------------|");
    println!("| &'a T           | covariant     | covariant          |");
    println!("| &'a mut T       | covariant     | INVARIANT          |");
    println!("| Box<T>          | —             | covariant          |");
    println!("| Vec<T>          | —             | covariant          |");
    println!("| UnsafeCell<T>   | —             | INVARIANT          |");
    println!("| Cell<T>         | —             | INVARIANT          |");
    println!("| fn(T) -> U      | —             | CONTRAVARIANT (T)  |");
    println!("|                 |               |   covariant (U)    |");
    println!("| *const T        | —             | covariant          |");
    println!("| *mut T          | —             | INVARIANT          |");
    println!();
    println!("Key patterns:");
    println!("  - Shared refs (&T)   -> covariant (safe to substitute)");
    println!("  - Mutable refs (&mut T) -> INVARIANT (must match exactly)");
    println!("  - Interior mutability (Cell, UnsafeCell) -> INVARIANT");
    println!("  - Function args      -> CONTRAVARIANT (inverted)");
    println!("  - Function returns   -> covariant (normal)");
}

// ============================================================
// SECTION 3: Why &mut T Is Invariant
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_why_mut_is_invariant() {
    println!("============================================================");
    println!("SECTION 3: Why &mut T Is Invariant");
    println!("============================================================");
    println!();
    println!("&mut T is INVARIANT over T — you cannot substitute");
    println!("&mut &'long str where &mut &'short str is expected.");
    println!();
    println!("WHY? Because allowing it enables use-after-free:");
    println!();
    println!("  fn assign<T>(input: &mut T, val: T) {{ *input = val; }}");
    println!();
    println!("  let mut hello: &'static str = \"hello\";");
    println!("  {{");
    println!("      let world = String::from(\"world\");");
    println!("      assign(&mut hello, &world);  // WOULD BE BUG!");
    println!("  }}  // world dropped here");
    println!("  println!(\"{{}}\", hello);  // use-after-free!");
    println!();
    println!("If &mut were covariant, &mut &'static str could accept");
    println!("&mut &'world str. Then assign() would write &world (short)");
    println!("into hello (expects 'static). After the block, world is");
    println!("dropped — but hello still references it. USE-AFTER-FREE!");
    println!();
    println!("INVARIANCE prevents this: T must be EXACTLY &'static str.");
    println!("&world is &'world str — not the same type. REJECTED.");
    println!();
    println!("Contrast with &T (covariant) — this IS safe:");

    // &T is covariant, so this works fine:
    let hello: &'static str = "hello";
    let world = String::from("world");
    let world_ref = &world;
    debug_shared(hello, world_ref); // hello coerced &'static -> &'world
    println!();
    println!("  &T is covariant -> &'static coerces to &'world -> OK");
    println!("  &mut T is invariant -> no coercion -> prevents bugs");
}

// ============================================================
// SECTION 4: Contravariance of Function Arguments
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_contravariance_of_function_args() {
    println!("============================================================");
    println!("SECTION 4: Contravariance of Function Arguments");
    println!("============================================================");
    println!();
    println!("Function arguments are CONTRAVARIANT:");
    println!("  fn(&'static str) <: fn(&'a str)");
    println!();
    println!("A function accepting &'static str CAN be used where");
    println!("a function accepting &'a str is expected.");
    println!("  (It accepts MORE, so it's safe for any 'a <= 'static)");
    println!();
    println!("But the REVERSE is NOT allowed:");
    println!("  fn(&'a str) is NOT a subtype of fn(&'static str)");
    println!("  (It might reject 'static-length references)");
    println!();
    println!("Why this matters — the store() example:");
    println!();
    println!("  fn store(input: &'static str)  // expects 'static");
    println!();
    println!("  // This is FINE — \"hello\" is 'static:");
    println!("  demo(\"hello\", store);");
    println!();
    println!("  // This would be a BUG if allowed:");
    println!("  // let smuggle = String::from(\"smuggle\");");
    println!("  // demo(&smuggle, store);  // &smuggle is NOT 'static");
    println!("  //   -> would store a short-lived ref in a static vec");
    println!("  //   -> use-after-free after smuggle is dropped");
    println!();
    println!("Contravariance ensures fn(&'static str) cannot be passed");
    println!("where fn(&'a str) is expected when 'a < 'static.");
    println!("The type system prevents the smuggle at COMPILE TIME.");
    println!();
    println!("SUMMARY of variance direction:");
    println!("  &'a T        : covariant over 'a  (longer -> shorter OK)");
    println!("  &'a mut T    : invariant over T   (must match exactly)");
    println!("  fn(T) -> U   : contravariant over T (inverted)");
    println!("  fn(T) -> U   : covariant over U    (normal)");
}

// ---- Helper functions ----

/// Accepts any &'a str — covariant, so &'static works too.
fn print_ref(s: &str) {
    println!("  print_ref: \"{}\"", s);
}

/// Shared ref version — covariant, both inputs coerced to same lifetime.
fn debug_shared<'a>(a: &'a str, b: &'a str) {
    println!("  debug_shared: a=\"{}\", b=\"{}\"", a, b);
}
