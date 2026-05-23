<!-- Context: development/backend/rust/errors/async | Priority: high | Version: 1.0 | Updated: 2026-05-24 -->

# Rust Async/Await Errors

Covers: async fn, await usage, generators, futures, Send bounds in async contexts.

---

## E0728 — `await` outside `async` context

`await` can only be used inside an `async fn` or `async` block.

```rust
// ❌
fn foo() {
    some_future().await; // error: await outside async
}

// ✅ — make the function async
async fn foo() {
    some_future().await;
}

// ✅ — or use an async block
fn foo() {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        some_future().await;
    });
}

// ✅ — or use block_on at the top level
fn main() {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            some_future().await;
        });
}
```

---

## E0752 — `main` cannot be `async`

`main` must be synchronous unless using a runtime macro.

```rust
// ❌
async fn main() { // error without runtime attribute
    do_async_work().await;
}

// ✅ — use tokio
#[tokio::main]
async fn main() {
    do_async_work().await;
}

// ✅ — use async-std
#[async_std::main]
async fn main() {
    do_async_work().await;
}

// ✅ — manual runtime
fn main() {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            do_async_work().await;
        });
}
```

---

## E0670 — `async fn` in Rust 2015

`async`/`await` requires Rust 2018 edition or later.

```rust
// In Cargo.toml:
// [package]
// edition = "2021"  ← set this (or "2018")

// ✅ — with edition = "2021" or "2018"
async fn foo() {}
```

---

## E0706 — `async fn` in trait (historical)

> Note: E0706 is no longer emitted in recent Rust versions. `async fn` in traits is now stable (Rust 1.75+).

```rust
// ✅ Rust 1.75+ — async fn in traits is stable
trait MyTrait {
    async fn fetch(&self) -> String;
}

// For older Rust or when you need Send bounds:
// Use the `async-trait` crate
use async_trait::async_trait;

#[async_trait]
trait MyTrait {
    async fn fetch(&self) -> String;
}

#[async_trait]
impl MyTrait for MyStruct {
    async fn fetch(&self) -> String {
        "hello".to_string()
    }
}
```

---

## E0733 — Recursive `async fn` without indirection

Async functions cannot call themselves directly because the future type would be infinitely sized.

```rust
// ❌
async fn count_down(n: u32) {
    if n > 0 {
        count_down(n - 1).await; // error: recursive async fn
    }
}

// ✅ — box the future
use std::future::Future;
use std::pin::Pin;

fn count_down(n: u32) -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(async move {
        if n > 0 {
            count_down(n - 1).await;
        }
    })
}

// ✅ — with the `async-recursion` crate
use async_recursion::async_recursion;

#[async_recursion]
async fn count_down(n: u32) {
    if n > 0 {
        count_down(n - 1).await;
    }
}
```

---

## E0626 — Borrow may still be in use at yield point

A reference is held across an `await` point, which may not be `Send`.

```rust
// ❌
async fn foo(mutex: &Mutex<i32>) {
    let guard = mutex.lock().unwrap(); // MutexGuard held
    some_async_fn().await;             // error: guard not Send
    println!("{}", *guard);
}

// ✅ — drop the guard before await
async fn foo(mutex: &Mutex<i32>) {
    {
        let guard = mutex.lock().unwrap();
        println!("{}", *guard);
    } // guard dropped here
    some_async_fn().await; // ok
}

// ✅ — use async-aware mutex (tokio::sync::Mutex)
use tokio::sync::Mutex;
async fn foo(mutex: &Mutex<i32>) {
    let guard = mutex.lock().await; // async lock, Send-safe
    some_async_fn().await;
    println!("{}", *guard);
}
```

---

## Common: Future not `Send`

When spawning tasks with `tokio::spawn`, the future must be `Send`.

```rust
// ❌
async fn my_task() {
    let rc = std::rc::Rc::new(5); // Rc is not Send
    some_async_fn().await;
    println!("{}", rc);
}
tokio::spawn(my_task()); // error: future not Send

// ✅ — use Arc instead of Rc
async fn my_task() {
    let arc = std::sync::Arc::new(5); // Arc is Send
    some_async_fn().await;
    println!("{}", arc);
}
tokio::spawn(my_task()); // ok

// ✅ — drop non-Send values before await
async fn my_task() {
    let result = {
        let rc = std::rc::Rc::new(5);
        *rc + 1
    }; // rc dropped here
    some_async_fn().await;
    println!("{}", result);
}
```

---

## Common: Blocking in async context

Calling blocking code inside async functions blocks the executor thread.

```rust
// ❌ — blocks the async runtime
async fn fetch_data() -> String {
    std::thread::sleep(std::time::Duration::from_secs(1)); // blocks!
    std::fs::read_to_string("file.txt").unwrap() // blocks!
}

// ✅ — use async equivalents
async fn fetch_data() -> String {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    tokio::fs::read_to_string("file.txt").await.unwrap()
}

// ✅ — or spawn_blocking for unavoidable blocking code
async fn fetch_data() -> String {
    tokio::task::spawn_blocking(|| {
        std::fs::read_to_string("file.txt").unwrap()
    })
    .await
    .unwrap()
}
```

---

## Async Quick Reference

```rust
// Basic async fn
async fn fetch() -> Result<String, Error> {
    let response = client.get(url).send().await?;
    Ok(response.text().await?)
}

// Spawn concurrent tasks
let (a, b) = tokio::join!(task_a(), task_b());

// Spawn independent task
let handle = tokio::spawn(async move { work().await });
let result = handle.await?;

// Timeout
use tokio::time::{timeout, Duration};
let result = timeout(Duration::from_secs(5), fetch()).await?;

// Select (first to complete wins)
tokio::select! {
    result = fetch_a() => handle_a(result),
    result = fetch_b() => handle_b(result),
}
```

---

## Related Context

- **Traits** → `traits.md` (async fn in traits, Send/Sync)
- **Lifetimes** → `lifetimes.md` (lifetimes in async contexts)
- **Actix patterns** → `../actix-patterns.md` (async handlers)
