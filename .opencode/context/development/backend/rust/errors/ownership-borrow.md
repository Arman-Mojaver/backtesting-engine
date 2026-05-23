<!-- Context: development/backend/rust/errors/ownership-borrow | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Ownership & Borrow Errors

Covers: move errors, borrow conflicts, mutability violations, uninitialized use.

---

## E0381 — Use of possibly uninitialized variable

Variable used before being assigned a value.

```rust
// ❌
let x: i32;
println!("{}", x); // error: use of possibly uninitialized `x`

// ✅
let x: i32 = 0;
println!("{}", x);

// ✅ — initialize in all branches
let x: i32;
if condition { x = 1; } else { x = 2; }
println!("{}", x);
```

---

## E0382 — Use after move

Variable used after its value was moved to another binding or function.

```rust
// ❌
let s = String::from("hello");
let t = s;       // s is moved here
println!("{}", s); // error: use of moved value

// ✅ Option 1: clone
let s = String::from("hello");
let t = s.clone();
println!("{}", s); // ok

// ✅ Option 2: borrow instead of move
let s = String::from("hello");
let t = &s;
println!("{}", s); // ok

// ✅ Option 3: implement Copy (for small types you control)
#[derive(Copy, Clone)]
struct Point { x: i32, y: i32 }
let p1 = Point { x: 1, y: 2 };
let p2 = p1; // copied, not moved
println!("{}", p1.x); // ok

// ✅ Option 4: shared ownership with Rc/Arc
use std::rc::Rc;
let s = Rc::new(String::from("hello"));
let t = Rc::clone(&s);
println!("{}", s); // ok
```

---

## E0383 — Use of partially moved value

A field was moved out of a struct, then the struct (or another field) was used.

```rust
// ❌
struct Foo { a: String, b: String }
let foo = Foo { a: "x".into(), b: "y".into() };
let a = foo.a; // partial move
println!("{}", foo.b); // error: partial move

// ✅ Move all fields you need before using the struct
let a = foo.a;
let b = foo.b;
// don't use foo after this

// ✅ Or use references
let a = &foo.a;
println!("{}", foo.b); // ok — only borrowed, not moved
```

---

## E0384 — Reassignment of immutable variable

```rust
// ❌
let x = 3;
x = 5; // error: cannot assign twice to immutable variable

// ✅
let mut x = 3;
x = 5;

// ✅ shadowing (creates new binding)
let x = 3;
let x = 5;
```

---

## E0386 / E0387 / E0388 / E0389 — Mutation of immutable data

```rust
// E0596: cannot borrow as mutable — not declared `mut`
// ❌
let x = 5;
let y = &mut x; // error

// ✅
let mut x = 5;
let y = &mut x;

// E0594: cannot assign to immutable reference
// ❌
let x = 5;
let r = &x;
*r = 6; // error

// ✅ — need &mut
let mut x = 5;
let r = &mut x;
*r = 6;
```

---

## E0499 — Mutable borrow more than once

Rust allows only one mutable borrow at a time.

```rust
// ❌
let mut i = 0;
let x = &mut i;
let y = &mut i; // error: cannot borrow `i` as mutable more than once
println!("{} {}", x, y);

// ✅ — use sequentially, not simultaneously
let mut i = 0;
{
    let x = &mut i;
    *x += 1;
} // x dropped here
let y = &mut i; // ok now
```

---

## E0502 — Borrow conflicts (mutable + immutable)

Cannot have a mutable borrow while an immutable borrow is active.

```rust
// ❌
fn bar(x: &mut i32) {}
fn foo(a: &mut i32) {
    let y = &a;    // immutable borrow
    bar(a);        // error: mutable borrow while immutable exists
    println!("{}", y);
}

// ✅ — use immutable borrow after mutable is done
fn foo(a: &mut i32) {
    bar(a);        // mutable borrow ends here
    let y = &a;    // ok
    println!("{}", y);
}
```

---

## E0505 — Move while borrowed

```rust
// ❌
struct Value {}
fn borrow(val: &Value) {}
fn eat(val: Value) {}

let x = Value{};
let ref_to_val = &x;
eat(x);              // error: cannot move `x` because it is borrowed
borrow(ref_to_val);

// ✅ Option 1: pass by reference
eat(&x);
borrow(ref_to_val);

// ✅ Option 2: drop borrow before move
borrow(ref_to_val);  // use borrow first
eat(x);              // then move

// ✅ Option 3: implement Copy
#[derive(Clone, Copy)]
struct Value {}
```

---

## E0507 — Move out of borrowed content

Cannot move a value out of a reference or borrowed struct field.

```rust
// ❌
use std::cell::RefCell;
struct Knight;
impl Knight { fn act(self) {} }

let x = RefCell::new(Knight);
x.borrow().act(); // error: cannot move out of `*x.borrow()`

// ✅ Option 1: take by reference
impl Knight { fn act(&self) {} }
x.borrow().act(); // ok

// ✅ Option 2: reclaim ownership
let x = RefCell::new(Knight);
let knight = x.into_inner();
knight.act();

// ✅ Option 3: implement Copy
#[derive(Clone, Copy)]
struct Knight;

// ✅ Option 4: use mem::replace for struct fields
use std::mem;
struct Cave { knight: Knight }
let mut cave = Cave { knight: Knight };
let borrowed = &mut cave;
mem::replace(&mut borrowed.knight, Knight).act();
```

---

## E0508 — Move out of array

Cannot move a non-Copy value out of an array by indexing.

```rust
// ❌
let arr = [String::from("a"), String::from("b")];
let s = arr[0]; // error: cannot move out of array

// ✅ clone
let s = arr[0].clone();

// ✅ use reference
let s = &arr[0];

// ✅ use into_iter to consume
for s in arr { /* s is owned */ }
```

---

## E0509 — Move out of type implementing Drop

Cannot move a field out of a type that implements `Drop`.

```rust
// ❌
struct Droppable { name: String }
impl Drop for Droppable { fn drop(&mut self) {} }

let d = Droppable { name: "x".into() };
let name = d.name; // error: cannot move out of type implementing Drop

// ✅ clone the field
let name = d.name.clone();

// ✅ use mem::ManuallyDrop to take ownership
use std::mem::ManuallyDrop;
let mut d = ManuallyDrop::new(Droppable { name: "x".into() });
let name = unsafe { ManuallyDrop::take(&mut d) }.name;
```

---

## E0515 — Return reference to local variable

See `lifetimes.md` → E0515.

---

## E0594 / E0596 — Cannot assign / borrow as mutable

```rust
// E0594: assign to immutable
// ❌
let x = 5;
// x = 6; // error

// ✅
let mut x = 5;
x = 6;

// E0596: borrow as mutable
// ❌
let s = String::new();
s.push('a'); // error: cannot borrow as mutable

// ✅
let mut s = String::new();
s.push('a');
```

---

## E0597 — Value dropped while borrowed

See `lifetimes.md` → E0597.

---

## Related Context

- **Lifetimes** → `lifetimes.md` (dangling refs, lifetime annotations)
- **Traits** → `traits.md` (Copy, Clone, Drop impls)
- **Actix patterns** → `../actix-patterns.md`
