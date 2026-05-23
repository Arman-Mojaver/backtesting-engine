<!-- Context: development/backend/rust/errors/modules | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Module & Visibility Errors

Covers: unresolved imports, visibility rules, crate resolution, private items.

---

## E0432 — Unresolved import

A `use` path cannot be resolved.

```rust
// ❌
use something::Foo; // error: unresolved import

// ✅ — check the path is correct
use std::collections::HashMap;

// ✅ — Rust 2018+: paths are relative to current module
// In module `foo::bar`, to import from sibling:
use super::sibling::Thing;
use crate::other_module::Thing;

// ✅ — Rust 2015: use self:: for relative paths
use self::something::Foo;

// ✅ — external crate: add to Cargo.toml first
// Cargo.toml: serde = "1"
use serde::Serialize;
```

---

## E0433 — Undeclared crate, module, or type

Using a name that doesn't exist in scope.

```rust
// ❌
let map = HashMap::new(); // error: use of undeclared type `HashMap`

// ✅
use std::collections::HashMap;
let map: HashMap<String, i32> = HashMap::new();

// ❌ — crate not in Cargo.toml
use tokio::runtime::Runtime; // error if tokio not in dependencies

// ✅ — add to Cargo.toml
// [dependencies]
// tokio = { version = "1", features = ["full"] }

// ❌ — module path wrong
use crate::models::user::User; // error if path doesn't exist

// ✅ — use crate:: prefix for absolute paths within crate
use crate::models::User;
```

---

## E0252 / E0254 / E0255 — Import name conflicts

Two imports bring the same name into scope.

```rust
// ❌
use std::fmt::Result;
use std::io::Result; // error: Result imported twice

// ✅ — alias one
use std::fmt::Result as FmtResult;
use std::io::Result as IoResult;

// ✅ — or use full paths inline
fn foo() -> std::fmt::Result { Ok(()) }
fn bar() -> std::io::Result<()> { Ok(()) }
```

---

## E0425 — Unresolved name

A name used in code doesn't exist in the current scope.

```rust
// ❌
println!("{}", undefined_var); // error

// ❌ — function not imported
let x = sqrt(4.0); // error: not in scope

// ✅
let x = f64::sqrt(4.0);
// or
use std::f64::consts;
```

---

## E0428 — Name defined multiple times

Two items in the same scope have the same name.

```rust
// ❌
mod foo {
    struct Bar;
    struct Bar; // error: Bar defined twice
}

// ❌
use std::fmt::Debug;
trait Debug {} // error: conflicts with imported Debug

// ✅ — rename one
use std::fmt::Debug as FmtDebug;
trait Debug {}
```

---

## E0432 / E0463 — Can't find crate

```rust
// ❌ in Cargo.toml is missing the dependency
use serde::Serialize; // error: can't find crate `serde`

// ✅ — add to Cargo.toml
// [dependencies]
// serde = { version = "1", features = ["derive"] }
```

---

## E0446 — Private type in public interface

A public function/trait exposes a private type.

```rust
// ❌
struct PrivateType;
pub fn expose() -> PrivateType { PrivateType } // error

// ✅ Option 1: make the type public
pub struct PrivateType;
pub fn expose() -> PrivateType { PrivateType }

// ✅ Option 2: restrict the function's visibility
pub(crate) fn expose() -> PrivateType { PrivateType }

// ✅ Option 3: return a trait object or public wrapper
pub struct PublicWrapper(PrivateType);
pub fn expose() -> PublicWrapper { PublicWrapper(PrivateType) }
```

---

## E0603 — Private item used

Attempting to use a private function, struct, field, or module.

```rust
// ❌
mod inner {
    struct Secret;
    fn hidden() {}
}
use inner::Secret; // error: Secret is private
inner::hidden();   // error: hidden is private

// ✅ — make items public
mod inner {
    pub struct Secret;
    pub fn hidden() {}
}

// ✅ — or access through a public API
mod inner {
    struct Secret { value: i32 }
    pub fn create() -> impl std::fmt::Debug { Secret { value: 42 } }
}
```

---

## E0616 — Field is private

Accessing a struct field that isn't public.

```rust
// ❌
mod foo {
    pub struct Bar { secret: i32 }
}
let b = foo::Bar { secret: 5 }; // error: field is private

// ✅ — add pub to field
mod foo {
    pub struct Bar { pub secret: i32 }
}

// ✅ — or provide constructor
mod foo {
    pub struct Bar { secret: i32 }
    impl Bar {
        pub fn new(secret: i32) -> Self { Bar { secret } }
        pub fn secret(&self) -> i32 { self.secret }
    }
}
```

---

## E0445 — Private trait in public interface

```rust
// ❌
trait PrivateTrait {}
pub fn foo<T: PrivateTrait>(x: T) {} // error

// ✅
pub trait PublicTrait {}
pub fn foo<T: PublicTrait>(x: T) {}

// ✅ — or restrict visibility
pub(crate) trait PrivateTrait {}
pub(crate) fn foo<T: PrivateTrait>(x: T) {}
```

---

## Visibility Quick Reference

```rust
pub struct Foo          // visible everywhere
pub(crate) struct Foo   // visible within this crate
pub(super) struct Foo   // visible to parent module
pub(in crate::foo) struct Foo  // visible within crate::foo
struct Foo              // private (visible only in this module)
```

---

## Module Path Quick Reference

```rust
use crate::foo::Bar;    // absolute path from crate root
use super::Bar;         // parent module
use self::Bar;          // current module (usually redundant)
use ::std::vec::Vec;    // explicit external crate (rare)
```

---

## Related Context

- **Types** → `types.md` (type not in scope E0412)
- **Cargo** → `../cargo.md` (adding dependencies)
