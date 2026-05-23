<!-- Context: development/backend/rust/errors/patterns | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Pattern Matching Errors

Covers: non-exhaustive matches, binding errors, irrefutable patterns, struct/tuple patterns.

---

## E0004 — Non-exhaustive match patterns

Not all possible values of the matched type are covered.

```rust
// ❌
enum Direction { North, South, East, West }
let d = Direction::North;
match d {
    Direction::North => println!("north"),
    Direction::South => println!("south"),
    // error: East and West not covered
}

// ✅ — cover all variants
match d {
    Direction::North => println!("north"),
    Direction::South => println!("south"),
    Direction::East  => println!("east"),
    Direction::West  => println!("west"),
}

// ✅ — or use wildcard
match d {
    Direction::North => println!("north"),
    _ => println!("other"),
}
```

---

## E0005 / E0007 / E0009 — Binding by reference and value in same pattern

Cannot mix `ref` bindings and move bindings in the same pattern.

```rust
// ❌
struct Foo { a: String, b: String }
let foo = Foo { a: "x".into(), b: "y".into() };
let Foo { a, ref b } = foo; // error: mix of move and ref

// ✅ — use all ref or all move
let Foo { ref a, ref b } = foo; // borrow both
let Foo { a, b } = foo;         // move both
```

---

## E0023 — Wrong number of fields in tuple struct pattern

```rust
// ❌
struct Point(i32, i32);
let Point(x) = Point(1, 2); // error: expected 2 fields, found 1

// ✅
let Point(x, y) = Point(1, 2);
let Point(x, _) = Point(1, 2); // ignore second field
```

---

## E0026 / E0027 — Struct pattern field mismatch

```rust
// E0026: extra fields in pattern
// ❌
struct Foo { x: i32 }
let Foo { x, y } = Foo { x: 1 }; // error: no field `y`

// ✅
let Foo { x } = Foo { x: 1 };

// E0027: missing fields in pattern
// ❌
struct Foo { x: i32, y: i32 }
let Foo { x } = Foo { x: 1, y: 2 }; // error: missing `y`

// ✅ — use .. to ignore remaining fields
let Foo { x, .. } = Foo { x: 1, y: 2 };
```

---

## E0408 — Variable not bound in all patterns

An `|` pattern must bind the same variables in each alternative.

```rust
// ❌
enum Foo { A(i32), B(i32), C }
match Foo::A(1) {
    Foo::A(x) | Foo::C => println!("{}", x), // error: x not bound in C branch
    Foo::B(x) => println!("{}", x),
}

// ✅ — bind in all alternatives
match Foo::A(1) {
    Foo::A(x) | Foo::B(x) => println!("{}", x),
    Foo::C => println!("C"),
}
```

---

## E0409 — Variable bound inconsistently across patterns

Same variable bound with different types or mutability in `|` alternatives.

```rust
// ❌
enum Foo { A(i32), B(u32) }
match Foo::A(1) {
    Foo::A(x) | Foo::B(x) => println!("{}", x), // error: x is i32 in A, u32 in B
}

// ✅ — ensure same type in all alternatives
// Either change the enum or handle separately
match Foo::A(1) {
    Foo::A(x) => println!("{}", x),
    Foo::B(x) => println!("{}", x),
}
```

---

## E0416 — Identifier bound more than once in pattern

```rust
// ❌
let (x, x) = (1, 2); // error: x bound twice

// ✅
let (x, y) = (1, 2);
let (x, _) = (1, 2); // ignore second
```

---

## E0527 / E0528 — Wrong number of elements in array/slice pattern

```rust
// E0527: array pattern
// ❌
let arr = [1, 2, 3];
let [a, b] = arr; // error: expected 3 elements, found 2

// ✅
let [a, b, c] = arr;
let [a, b, ..] = arr; // rest pattern (slice patterns)

// E0528: slice pattern
// ❌
let slice: &[i32] = &[1, 2, 3];
if let [a, b] = slice { } // error: slice has 3 elements

// ✅
if let [a, b, c] = slice { }
if let [a, b, ..] = slice { } // match 2+ elements
if let [a, ..] = slice { }    // match 1+ elements
```

---

## E0530 — Pattern binding shadows something else

A pattern binding has the same name as a constant or enum variant in scope.

```rust
// ❌
const FOO: i32 = 5;
let x = 5;
match x {
    FOO => println!("matched"), // error: FOO is a const, not a binding
    _ => {}
}
// Actually this is valid — const patterns work. The error occurs when
// you shadow a unit struct/variant:

enum Status { Active, Inactive }
use Status::*;
let s = Active;
match s {
    Active => {},
    Inactive => {},
}
// If you accidentally shadow: rename your binding variable
```

---

## E0579 — Lower range bound greater than upper

```rust
// ❌
match 5u8 {
    10..=5 => {}, // error: lower bound 10 > upper bound 5
    _ => {}
}

// ✅
match 5u8 {
    5..=10 => {},
    _ => {}
}
```

---

## E0162 / E0165 — Irrefutable pattern in `if let` / `while let`

The pattern always matches, making `if let` / `while let` pointless.

```rust
// ❌
if let x = 5 { println!("{}", x); } // warning/error: irrefutable

// ✅ — use let directly
let x = 5;
println!("{}", x);

// if let is for refutable patterns (Option, Result, enums with multiple variants)
if let Some(x) = maybe_value { println!("{}", x); }
```

---

## Quick Reference: Pattern Syntax

```rust
// Tuple struct
let Point(x, y) = p;
let Point(x, ..) = p;  // ignore rest

// Struct
let Foo { x, y } = f;
let Foo { x, .. } = f; // ignore rest

// Enum
match opt {
    Some(x) => {},
    None => {},
}

// Or patterns
match x {
    1 | 2 | 3 => {},
    _ => {},
}

// Range
match x {
    1..=5 => {},
    _ => {},
}

// Guard
match x {
    n if n > 0 => {},
    _ => {},
}

// Binding + pattern
match x {
    n @ 1..=5 => println!("got {}", n),
    _ => {},
}
```

---

## Related Context

- **Types** → `types.md` (type mismatches in patterns)
- **Ownership** → `ownership-borrow.md` (move vs ref in patterns)
