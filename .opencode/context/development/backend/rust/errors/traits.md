<!-- Context: development/backend/rust/errors/traits | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Trait Errors

Covers: trait bounds, impl conflicts, object safety, orphan rules, derive errors.

---

## E0034 — Multiple applicable methods (ambiguous)

Two or more traits in scope provide a method with the same name.

```rust
// ❌
trait Foo { fn method(&self) -> u8; }
trait Bar { fn method(&self) -> u8; }
struct Baz;
impl Foo for Baz { fn method(&self) -> u8 { 1 } }
impl Bar for Baz { fn method(&self) -> u8 { 2 } }

let b = Baz;
b.method(); // error: ambiguous

// ✅ — use fully qualified syntax
Foo::method(&b);
Bar::method(&b);
// or
<Baz as Foo>::method(&b);
```

---

## E0038 — Trait not object-safe

A trait cannot be used as `dyn Trait` because it violates object safety rules.

```rust
// ❌ — method returns Self
trait Clone2 {
    fn clone(&self) -> Self; // Self in return position → not object-safe
}
let _: Box<dyn Clone2>; // error

// ❌ — method has generic type params
trait Foo {
    fn bar<T>(&self, x: T); // generic method → not object-safe
}

// ✅ — remove Self from return, use Box<Self> or associated type
trait Cloneable {
    fn clone_box(&self) -> Box<dyn Cloneable>;
}

// ✅ — use generics instead of trait objects
fn use_foo<T: Foo>(x: &T) { x.bar(1i32); }
```

**Object safety rules**: A trait is object-safe if:
- No method returns `Self`
- No method has generic type parameters
- No associated constants (unless default)
- Trait does not require `Sized`

---

## E0046 — Missing trait item in impl

An `impl` block is missing one or more required items from the trait.

```rust
// ❌
trait Animal {
    fn name(&self) -> &str;
    fn sound(&self) -> &str;
}
struct Dog;
impl Animal for Dog {
    fn name(&self) -> &str { "Dog" }
    // error: missing `sound`
}

// ✅
impl Animal for Dog {
    fn name(&self) -> &str { "Dog" }
    fn sound(&self) -> &str { "Woof" }
}
```

---

## E0117 — Orphan rule violation

Cannot implement a foreign trait for a foreign type. At least one of trait or type must be local.

```rust
// ❌
impl std::fmt::Display for Vec<i32> {} // both foreign

// ✅ — wrap in a local newtype
struct MyVec(Vec<i32>);
impl std::fmt::Display for MyVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

// ✅ — define a local trait
trait MyDisplay { fn show(&self) -> String; }
impl MyDisplay for Vec<i32> { fn show(&self) -> String { format!("{:?}", self) } }
```

---

## E0119 — Conflicting trait implementations

Two `impl` blocks cover the same type for the same trait.

```rust
// ❌
trait MyTrait { fn get(&self) -> usize; }
impl<T> MyTrait for T { fn get(&self) -> usize { 0 } }
struct Foo;
impl MyTrait for Foo { fn get(&self) -> usize { 1 } } // error: conflicts with blanket impl

// ✅ — remove the blanket impl or the specific impl
// If you need both, use specialization (nightly only)
// Otherwise, restructure to avoid overlap
```

---

## E0184 — Cannot implement both `Copy` and `Drop`

```rust
// ❌
#[derive(Copy, Clone)]
struct Foo;
impl Drop for Foo { fn drop(&mut self) {} } // error

// ✅ — choose one: either Copy (no destructor) or Drop (no copy)
// If you need cleanup, use Drop and clone explicitly
#[derive(Clone)]
struct Foo;
impl Drop for Foo { fn drop(&mut self) {} }
```

---

## E0204 — Cannot derive `Copy` (contains non-Copy field)

```rust
// ❌
#[derive(Copy, Clone)]
struct Foo { s: String } // String is not Copy

// ✅ Option 1: remove Copy derive, use Clone
#[derive(Clone)]
struct Foo { s: String }

// ✅ Option 2: use Copy-able types
#[derive(Copy, Clone)]
struct Foo { x: i32, y: f64 } // all Copy
```

---

## E0225 — Only auto-traits allowed after first trait bound in `dyn`

```rust
// ❌
use std::fmt;
let _: Box<dyn fmt::Debug + fmt::Display>; // error: two non-auto traits

// ✅ — combine into one trait
trait DebugDisplay: fmt::Debug + fmt::Display {}
impl<T: fmt::Debug + fmt::Display> DebugDisplay for T {}
let _: Box<dyn DebugDisplay>;

// ✅ — auto-traits (Send, Sync, Unpin) are allowed alongside one trait
let _: Box<dyn fmt::Debug + Send + Sync>; // ok
```

---

## E0271 — Type mismatch in trait associated type

```rust
// ❌
trait Converter {
    type Output;
    fn convert(&self) -> Self::Output;
}
struct Foo;
impl Converter for Foo {
    type Output = String;
    fn convert(&self) -> i32 { 0 } // error: expected String, found i32
}

// ✅
impl Converter for Foo {
    type Output = i32;
    fn convert(&self) -> i32 { 0 }
}
```

---

## E0277 — Trait bound not satisfied

The type doesn't implement a required trait.

```rust
// ❌
fn print<T>(x: T) {
    println!("{:?}", x); // error: T doesn't implement Debug
}

// ✅ — add the bound
fn print<T: std::fmt::Debug>(x: T) {
    println!("{:?}", x);
}

// ❌ — common: missing Send/Sync for async
async fn spawn_task<T>(x: T) {
    tokio::spawn(async move { /* use x */ }); // error: T not Send
}

// ✅
async fn spawn_task<T: Send + 'static>(x: T) {
    tokio::spawn(async move { /* use x */ });
}
```

**Common missing bounds**:
| Situation | Required bound |
|-----------|---------------|
| `println!("{:?}", x)` | `T: Debug` |
| `println!("{}", x)` | `T: Display` |
| `x == y` | `T: PartialEq` |
| `x < y` | `T: PartialOrd` |
| `tokio::spawn(...)` | `T: Send + 'static` |
| `Arc<T>` across threads | `T: Send + Sync` |
| Serialize with serde | `T: Serialize` |

---

## E0283 — Ambiguous trait implementation

Multiple implementations could apply; compiler can't choose.

```rust
// ❌
trait Into2<T> { fn into2(self) -> T; }
impl Into2<u32> for u8 { fn into2(self) -> u32 { self as u32 } }
impl Into2<u64> for u8 { fn into2(self) -> u64 { self as u64 } }

let x: u8 = 5;
let y = x.into2(); // error: ambiguous — u32 or u64?

// ✅ — annotate the target type
let y: u32 = x.into2();
// or use turbofish
let y = x.into2::<u32>();
```

---

## E0562 — `impl Trait` in non-return position

`impl Trait` can only be used in function return types and argument positions (not in struct fields, type aliases in some contexts, etc.).

```rust
// ❌
struct Foo {
    x: impl std::fmt::Debug, // error
}

// ✅ — use a generic parameter
struct Foo<T: std::fmt::Debug> {
    x: T,
}

// ✅ — or box the trait object
struct Foo {
    x: Box<dyn std::fmt::Debug>,
}
```

---

## E0782 / E0783 — Trait objects must include `dyn`

In Rust 2021+, bare trait objects require `dyn`.

```rust
// ❌ (Rust 2021)
fn foo(x: &std::fmt::Debug) {} // error: missing `dyn`

// ✅
fn foo(x: &dyn std::fmt::Debug) {}
fn foo(x: Box<dyn std::fmt::Debug>) {}
```

---

## Related Context

- **Generics** → `generics.md` (associated types, type params)
- **Ownership** → `ownership-borrow.md` (Copy, Clone, Drop)
