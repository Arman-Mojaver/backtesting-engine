<!-- Context: development/backend/rust/errors/lifetimes | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Lifetime Errors

Covers: missing lifetime annotations, elision rules, dangling references, lifetime bounds.

---

## E0106 — Missing lifetime specifier

A reference in a struct, enum, type alias, or function return needs an explicit lifetime.

```rust
// ❌
struct Foo { x: &bool }
type MyStr = &str;
fn first(x: &str, y: &str) -> &str { x }

// ✅ — annotate the struct
struct Foo<'a> { x: &'a bool }
type MyStr<'a> = &'a str;

// ✅ — lifetime elision rules for functions:
// Rule 1: one input lifetime → output gets same lifetime
fn first(x: &str) -> &str { x }  // ok, elided

// Rule 2: multiple inputs + &self → output gets self's lifetime
impl Foo<'_> {
    fn get(&self, other: &str) -> &str { "x" } // ok
}

// Rule 3: multiple inputs, no self → must be explicit
fn first<'a>(x: &'a str, y: &str) -> &'a str { x }
```

---

## E0261 — Undeclared lifetime

Using a lifetime name that hasn't been declared.

```rust
// ❌
fn foo(x: &'a str) -> &'a str { x } // 'a not declared

// ✅
fn foo<'a>(x: &'a str) -> &'a str { x }
```

---

## E0262 — Invalid lifetime name

Lifetime names must start with `'` followed by an identifier. `'static` and `'_` are reserved.

```rust
// ❌
fn foo<'static>(x: &'static str) {} // 'static is reserved

// ✅
fn foo<'a>(x: &'a str) {}
fn foo(x: &'static str) {} // use 'static directly
```

---

## E0263 — Lifetime declared twice

```rust
// ❌
fn foo<'a, 'a>(x: &'a str) {} // error: 'a declared twice

// ✅
fn foo<'a>(x: &'a str) {}
```

---

## E0309 — Type must outlive lifetime

A type used in a context requires it to outlive a specific lifetime.

```rust
// ❌
struct Ref<'a, T> { r: &'a T }
// If T contains references, they must outlive 'a

// ✅ — add bound
struct Ref<'a, T: 'a> { r: &'a T }

// In modern Rust (1.31+), this is often inferred automatically
struct Ref<'a, T> { r: &'a T } // compiler infers T: 'a
```

---

## E0310 — Type must outlive `'static`

```rust
// ❌
fn needs_static<T: 'static>(x: T) {}
fn foo<T>(x: T) {
    needs_static(x); // error: T may not be 'static
}

// ✅
fn foo<T: 'static>(x: T) {
    needs_static(x);
}
```

---

## E0373 — Closure outlives captured variable

A closure captures a reference to a local variable but the closure may outlive it.

```rust
// ❌
fn make_adder(x: i32) -> impl Fn(i32) -> i32 {
    |y| x + y // error if x is a reference that doesn't live long enough
}

// ✅ — use `move` to take ownership
fn make_adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y
}

// ✅ — for references, ensure lifetime is long enough
fn make_adder<'a>(x: &'a i32) -> impl Fn(i32) -> i32 + 'a {
    move |y| x + y
}
```

---

## E0476 / E0477 / E0478 — Lifetime bound not satisfied

A lifetime doesn't outlive the required scope.

```rust
// ❌
fn foo<'a, 'b>(x: &'a str, y: &'b str) -> &'a str {
    y // error: 'b doesn't necessarily outlive 'a
}

// ✅ — add bound 'b: 'a (b outlives a)
fn foo<'a, 'b: 'a>(x: &'a str, y: &'b str) -> &'a str {
    y
}
```

---

## E0491 — Reference has longer lifetime than referent

```rust
// ❌
struct Foo<'a> { x: &'a i32 }
fn foo<'a>(x: &'a Foo<'a>) -> &'a i32 { x.x }
// The reference to Foo and the reference inside Foo share 'a,
// which can cause issues

// ✅ — use separate lifetimes
fn foo<'a, 'b: 'a>(x: &'a Foo<'b>) -> &'a i32 { x.x }
```

---

## E0495 — Cannot infer lifetime due to conflicting requirements

```rust
// ❌
fn foo<'a>(x: &'a str, y: &str) -> &'a str {
    if true { x } else { y } // error: y's lifetime conflicts
}

// ✅ — give both the same lifetime
fn foo<'a>(x: &'a str, y: &'a str) -> &'a str {
    if true { x } else { y }
}
```

---

## E0515 — Return reference to local variable

Cannot return a reference to a value that will be dropped at end of function.

```rust
// ❌
fn get_str() -> &str {
    let s = String::from("hello");
    &s // error: s dropped at end of function
}

fn get_iter() -> impl Iterator<Item = i32> {
    let v = vec![1, 2, 3];
    v.iter() // error: v dropped
}

// ✅ — return owned value
fn get_str() -> String {
    String::from("hello")
}

fn get_iter() -> impl Iterator<Item = i32> {
    vec![1, 2, 3].into_iter() // consumes vec, no dangling ref
}

// ✅ — return 'static reference
fn get_str() -> &'static str {
    "hello" // string literals are 'static
}
```

---

## E0521 — Borrowed data escapes outside closure

```rust
// ❌
fn foo<'a>(x: &'a str) -> impl FnOnce() + 'static {
    move || println!("{}", x) // error: x doesn't live 'static
}

// ✅ — relax the 'static bound
fn foo<'a>(x: &'a str) -> impl FnOnce() + 'a {
    move || println!("{}", x)
}

// ✅ — clone to get owned data
fn foo(x: &str) -> impl FnOnce() + 'static {
    let owned = x.to_owned();
    move || println!("{}", owned)
}
```

---

## E0597 — Value dropped while borrowed

A value is dropped while a borrow of it is still active.

```rust
// ❌
struct Foo<'a> { x: Option<&'a u32> }
let mut foo = Foo { x: None };
{
    let y = 0;
    foo.x = Some(&y); // y borrowed here
}                     // y dropped here
println!("{:?}", foo.x); // error: y doesn't live long enough

// ✅ — extend y's scope
let mut foo = Foo { x: None };
let y = 0;
foo.x = Some(&y);
println!("{:?}", foo.x); // ok, y still alive

// ✅ — store owned value instead
struct Foo { x: Option<u32> }
let mut foo = Foo { x: None };
{
    let y = 0;
    foo.x = Some(y); // copy, no borrow
}
println!("{:?}", foo.x); // ok
```

---

## E0716 — Temporary dropped while borrowed

A temporary value is dropped before the borrow ends.

```rust
// ❌
fn foo() -> i32 { 22 }
fn bar(x: &i32) -> &i32 { x }
let p = bar(&foo()); // temporary from foo() dropped after this line
let q = *p;          // error: p is dangling

// ✅ — bind the temporary to a variable
let value = foo();
let p = bar(&value);
let q = *p; // ok

// ✅ — or bind the reference directly (compiler extends lifetime)
let value = &foo(); // compiler extends temp lifetime to end of block
let p = bar(value);
let q = *p; // ok
```

---

## E0637 — `'_` not allowed here

The anonymous lifetime `'_` cannot be used in certain positions.

```rust
// ❌
struct Foo<'_> { x: &'_ str } // error in struct definition

// ✅
struct Foo<'a> { x: &'a str }

// '_ is valid in function signatures (elision placeholder)
fn foo(x: &'_ str) -> &'_ str { x } // ok
```

---

## Lifetime Elision Rules (Quick Reference)

```
1. Each elided lifetime in input gets a distinct lifetime parameter.
2. If there is exactly one input lifetime, it is assigned to all output lifetimes.
3. If one of the inputs is &self or &mut self, its lifetime is assigned to all outputs.
```

```rust
fn first(x: &str) -> &str              // → fn first<'a>(x: &'a str) -> &'a str
fn split(x: &str, y: &str) -> &str     // ❌ ambiguous — must annotate
impl Foo { fn get(&self) -> &str }     // → fn get<'a>(&'a self) -> &'a str
```

---

## Related Context

- **Ownership/borrow** → `ownership-borrow.md`
- **Generics** → `generics.md` (lifetime params in generic types)
