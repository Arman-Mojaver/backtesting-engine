<!-- Context: development/backend/rust/errors/types | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Type Errors

Covers: type mismatches, casts, inference failures, struct/enum errors, misc compiler errors.

---

## E0308 — Type mismatch

Expected one type, received another. The most common Rust error.

```rust
// ❌
fn add_one(x: i32) -> i32 { x + 1 }
add_one("not a number"); // error: expected i32, found &str

let x: f32 = "not a float"; // error

if "not a bool" { } // error: expected bool

// ✅ — use the correct type
add_one(5);
let x: f32 = 3.14;
if true { }

// Common: missing return value
fn foo() -> i32 {
    let x = 5;
    // error: expected i32, found ()  ← missing `x` at end
    x  // ✅ no semicolon = return value
}

// Common: Option/Result mismatch
fn get() -> Option<i32> { Some(5) }
let x: i32 = get(); // error: expected i32, found Option<i32>
let x: i32 = get().unwrap(); // ✅
let x: i32 = get().unwrap_or(0); // ✅
```

---

## E0054 — Invalid cast

A `as` cast between incompatible types.

```rust
// ❌
let x = true;
let y = x as f32; // error: cannot cast bool to f32

// ✅ — convert explicitly
let y = if x { 1.0f32 } else { 0.0f32 };

// Valid casts:
let x: i32 = 5;
let y = x as f64;   // numeric widening ✅
let y = x as u8;    // numeric truncation ✅
let p = &x as *const i32; // ref to raw pointer ✅
```

---

## E0072 — Recursive type has infinite size

A struct/enum references itself without indirection.

```rust
// ❌
struct List {
    value: i32,
    next: Option<List>, // infinite size
}

// ✅ — add indirection with Box
struct List {
    value: i32,
    next: Option<Box<List>>,
}

// ✅ — or use Rc for shared ownership
use std::rc::Rc;
struct List {
    value: i32,
    next: Option<Rc<List>>,
}
```

---

## E0282 — Type annotations needed

The compiler cannot infer the type; annotation required.

```rust
// ❌
let x = []; // error: type of elements unknown
let y = Vec::new(); // error: element type unknown

// ✅ — annotate the variable
let x: [i32; 0] = [];
let y: Vec<i32> = Vec::new();

// ✅ — or use turbofish
let y = Vec::<i32>::new();

// ✅ — or provide context
let mut y = Vec::new();
y.push(1i32); // compiler infers Vec<i32>
```

---

## E0317 — `if` without `else` returns non-unit

An `if` expression used as a value must have an `else` branch.

```rust
// ❌
let x = if true { 5 }; // error: expected (), found integer

// ✅
let x = if true { 5 } else { 0 };

// ✅ — if used as statement (not value), else not needed
if true { println!("yes"); }
```

---

## E0391 — Type dependency cycle

Two types or traits depend on each other in a cycle.

```rust
// ❌
trait A: B {}
trait B: A {} // error: cycle

// ✅ — break the cycle with an intermediate trait or restructure
trait Base {}
trait A: Base {}
trait B: Base {}
```

---

## E0412 — Type not in scope

Using a type that hasn't been imported or defined.

```rust
// ❌
let map: HashMap<String, i32> = HashMap::new(); // error: not in scope

// ✅
use std::collections::HashMap;
let map: HashMap<String, i32> = HashMap::new();
```

---

## E0422 / E0423 — Does not name a struct / Expected function found struct

```rust
// E0422: using struct name as value
// ❌
struct Foo { x: i32 }
let f = Foo; // error: expected value, found struct

// ✅
let f = Foo { x: 5 };

// E0423: calling struct like a function
// ❌
let f = Foo(5); // error: Foo is a struct, not a tuple struct

// ✅ — use struct literal syntax
let f = Foo { x: 5 };
// or define as tuple struct
struct Foo(i32);
let f = Foo(5); // ok
```

---

## E0424 — `self` not available in static method

```rust
// ❌
struct Foo;
impl Foo {
    fn static_method() {
        println!("{:?}", self); // error: no `self` in static method
    }
}

// ✅ — add self parameter
impl Foo {
    fn instance_method(&self) {
        println!("ok");
    }
    fn static_method() {
        println!("no self needed");
    }
}
```

---

## E0435 — Non-constant value in constant expression

```rust
// ❌
let x = 5;
const Y: i32 = x; // error: x is not const

// ✅
const X: i32 = 5;
const Y: i32 = X;

// ✅ — or use static
static X: i32 = 5;
```

---

## E0512 — Transmute with different sizes

`std::mem::transmute` requires source and destination to have the same size.

```rust
// ❌
let x: u32 = 5;
let y: u64 = unsafe { std::mem::transmute(x) }; // error: different sizes

// ✅ — use a proper cast
let y: u64 = x as u64;

// ✅ — if you truly need transmute, ensure sizes match
let x: [u8; 4] = [0, 0, 0, 5];
let y: u32 = unsafe { std::mem::transmute(x) }; // ok: both 4 bytes
```

---

## E0571 — `break` with value outside `loop`

`break` with a value is only valid in `loop`, not `while` or `for`.

```rust
// ❌
while true {
    break 5; // error
}

// ✅ — use loop
let result = loop {
    break 5; // ok
};
```

---

## E0599 — Method not found

The method doesn't exist on the type, or the required trait isn't in scope.

```rust
// ❌
let v = vec![1, 2, 3];
v.frobnicate(); // error: no method named `frobnicate`

// Common cause: trait not imported
let s = "hello";
s.is_empty(); // ok — inherent method
// But for trait methods:
use std::io::Write;
let mut buf = Vec::new();
buf.write_all(b"hello").unwrap(); // needs `use std::io::Write`

// ❌ — wrong type
let x: i32 = 5;
x.len(); // error: i32 has no len()

// ✅ — check the type and available methods in docs
```

---

## E0605 — Non-primitive cast

Using `as` to cast between non-primitive types.

```rust
// ❌
struct Foo;
struct Bar;
let f = Foo;
let b = f as Bar; // error

// ✅ — implement From/Into
impl From<Foo> for Bar {
    fn from(_: Foo) -> Bar { Bar }
}
let b = Bar::from(Foo);
let b: Bar = Foo.into();
```

---

## E0614 — Cannot dereference non-pointer

```rust
// ❌
let x = 5i32;
let y = *x; // error: x is not a reference

// ✅ — only deref references/pointers
let x = 5i32;
let r = &x;
let y = *r; // ok
```

---

## E0618 — Expected function, found non-function

Calling something that isn't callable.

```rust
// ❌
struct Foo;
let f = Foo;
f(); // error: Foo is not a function

// ✅ — implement Fn trait or use a method
impl std::ops::FnOnce<()> for Foo {
    type Output = ();
    extern "rust-call" fn call_once(self, _: ()) {}
}
// or just call a method
Foo.some_method();
```

---

## Related Context

- **Traits** → `traits.md` (trait bounds, From/Into)
- **Ownership** → `ownership-borrow.md` (move/borrow)
- **Generics** → `generics.md` (type inference with generics)
