<!-- Context: development/backend/rust/errors/generics | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Generics & Associated Type Errors

Covers: wrong number of type args, unconstrained params, associated types, const generics.

---

## E0107 — Wrong number of generic arguments

```rust
// ❌
struct Pair<A, B>(A, B);
let p: Pair<i32> = Pair(1, 2); // error: expected 2 type args, found 1

// ✅
let p: Pair<i32, f64> = Pair(1, 2.0);

// ❌ — too many
fn foo<T>(x: T) {}
foo::<i32, f64>(5); // error: expected 1 type arg, found 2

// ✅
foo::<i32>(5);
foo(5i32); // let inference work
```

---

## E0109 — Type argument provided for non-generic type

```rust
// ❌
let x: i32<u8> = 5; // error: i32 is not generic

// ✅
let x: i32 = 5;
```

---

## E0207 — Unconstrained type/lifetime parameter in impl

A type or lifetime param on `impl` doesn't appear in the implementing type or trait.

```rust
// ❌
struct Foo;
impl<T: Default> Foo {
    fn get(&self) -> T { T::default() }
    // error: T not constrained by impl
}

// ✅ Option 1: move T to the method
impl Foo {
    fn get<T: Default>(&self) -> T { T::default() }
}

// ✅ Option 2: add T to the struct
struct Foo<T>(std::marker::PhantomData<T>);
impl<T: Default> Foo<T> {
    fn get(&self) -> T { T::default() }
}

// ❌ — unconstrained in trait impl
trait Maker { type Item; fn make(&mut self) -> Self::Item; }
struct FooMaker;
impl<T: Default> Maker for FooMaker { // T not in FooMaker or Maker
    type Item = T;
    fn make(&mut self) -> T { T::default() }
}

// ✅ — add T to FooMaker
struct FooMaker<T>(std::marker::PhantomData<T>);
impl<T: Default> Maker for FooMaker<T> {
    type Item = T;
    fn make(&mut self) -> T { T::default() }
}
```

---

## E0220 — Unknown associated type in trait bound

```rust
// ❌
trait Foo {}
fn bar<T: Foo<Output = i32>>(x: T) {} // error: Foo has no associated type Output

// ✅ — use the correct associated type name
trait Foo { type Result; }
fn bar<T: Foo<Result = i32>>(x: T) {}
```

---

## E0221 / E0222 / E0223 — Ambiguous associated type

Multiple supertraits define the same associated type.

```rust
// ❌
trait A { type Item; }
trait B { type Item; }
trait C: A + B {}
fn foo<T: C>(x: T::Item) {} // error: ambiguous — A::Item or B::Item?

// ✅ — use fully qualified syntax
fn foo<T: C>(x: <T as A>::Item) {}
fn foo<T: C>(x: <T as B>::Item) {}
```

---

## E0229 — Associated type binding in non-trait path

```rust
// ❌
struct Foo<T: Iterator<Item = i32>>(T); // ok in bounds
let x: Vec<Item = i32>; // error: Vec is not a trait

// ✅ — associated type bindings only in trait bounds
fn foo<T: Iterator<Item = i32>>(iter: T) {}
```

---

## E0392 — Type parameter never used

A type parameter declared on a struct/enum is never used in the body.

```rust
// ❌
struct Foo<T> { x: i32 } // error: T never used

// ✅ Option 1: use the type
struct Foo<T> { x: T }

// ✅ Option 2: use PhantomData
use std::marker::PhantomData;
struct Foo<T> {
    x: i32,
    _phantom: PhantomData<T>,
}
```

---

## E0401 — Use of type parameter from outer function

Cannot use a type parameter from an outer function in a nested function.

```rust
// ❌
fn outer<T>() {
    fn inner(x: T) {} // error: T from outer not in scope
}

// ✅ Option 1: add T to inner
fn outer<T>() {
    fn inner<T>(x: T) {}
}

// ✅ Option 2: use a closure (closures can capture outer generics)
fn outer<T: std::fmt::Debug>() {
    let inner = |x: T| println!("{:?}", x);
}
```

---

## E0403 — Two type parameters with same name

```rust
// ❌
fn foo<T, T>(x: T, y: T) {} // error: T declared twice

// ✅
fn foo<T, U>(x: T, y: U) {}
```

---

## E0575 — Expected associated type, found something else

```rust
// ❌
trait Foo { type Bar; }
fn baz<T: Foo>() -> T::Bar where T::Bar: i32 {} // error: i32 is not a trait

// ✅ — use correct syntax
fn baz<T: Foo>() -> T::Bar where T::Bar: std::fmt::Debug {}
```

---

## E0632 — Explicit generic arg after `..`

```rust
// ❌
foo::<.., i32>(); // error: explicit arg after ..

// ✅ — don't mix .. with explicit args this way
foo::<i32>();
```

---

## E0747 — Generic argument order

Lifetime args must come before type args, which come before const args.

```rust
// ❌
struct Foo<T, 'a>(&'a T); // error: lifetime after type

// ✅
struct Foo<'a, T>(&'a T);

// Order: lifetimes → types → consts
struct Bar<'a, T, const N: usize>(&'a [T; N]);
```

---

## PhantomData Quick Reference

Use `PhantomData<T>` when a struct logically "owns" or "uses" a type but doesn't store it directly:

```rust
use std::marker::PhantomData;

// Marks that Foo "owns" T (for drop check)
struct Foo<T> { ptr: *mut T, _owns: PhantomData<T> }

// Marks that Foo is covariant over T
struct Foo<T> { _marker: PhantomData<T> }

// Marks that Foo is contravariant over T (rare)
struct Foo<T> { _marker: PhantomData<fn(T)> }
```

---

## Related Context

- **Traits** → `traits.md` (trait bounds, associated types)
- **Lifetimes** → `lifetimes.md` (lifetime params in generics)
- **Types** → `types.md` (type inference E0282)
