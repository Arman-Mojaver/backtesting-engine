<!-- Context: standards/rust-blueprint | Priority: high | Version: 1.2 | Updated: 2026-11-01 -->

# Production Rust Blueprint

A senior engineer's reference for writing performant, testable, and maintainable Rust in this backtesting backend. Every pattern here is chosen for production correctness — not tutorial simplicity.

---

## Table of Contents

1. [Traits & Polymorphism](#1-traits--polymorphism)
2. [Generics & Type Parameters](#2-generics--type-parameters)
3. [Error Handling](#3-error-handling)
4. [Functional Patterns & Iterators](#4-functional-patterns--iterators)
5. [Memory Management & Smart Pointers](#5-memory-management--smart-pointers)
6. [Lifetimes](#6-lifetimes)
7. [Async & Concurrency](#7-async--concurrency)
8. [Architecture: Ports & Adapters](#8-architecture-ports--adapters)
9. [Anti-Patterns](#9-anti-patterns)
10. [Tooling Checklist](#10-tooling-checklist)

---

## 1. Traits & Polymorphism

### Define behaviour, not data

Traits are the primary abstraction mechanism. Define them narrow — one responsibility per trait. This keeps implementations composable and testable.

```rust
/// Computes a scalar metric from a slice of operation points.
/// Implement this for each metric (Sharpe, max drawdown, win rate, etc.)
pub trait Metric {
    fn compute(&self, points: &[OperationPoint]) -> f64;
}

/// Persists and retrieves operation points.
/// Implement this for Postgres, in-memory (tests), CSV, etc.
pub trait OperationPointRepository {
    fn find_by_instrument(&self, instrument: &str) -> Vec<OperationPoint>;
    fn save(&mut self, point: OperationPoint) -> Result<(), RepositoryError>;
}
```

### Static dispatch (generics) vs dynamic dispatch (trait objects)

**Use generics (`impl Trait` / `<T: Trait>`) when:**
- The concrete type is known at compile time
- You want zero-cost abstraction (no vtable, fully inlined)
- Performance is critical (hot paths: indicator computation, backtesting loops)

```rust
// ✅ Zero-cost: monomorphised at compile time, no heap allocation
fn run_backtest<M: Metric>(metric: &M, points: &[OperationPoint]) -> f64 {
    metric.compute(points)
}

// ✅ `impl Trait` in return position: caller gets a concrete type, compiler inlines
fn default_metric() -> impl Metric {
    SharpeRatio::new(252)
}
```

**Use trait objects (`&dyn Trait` / `Box<dyn Trait>`) when:**
- The concrete type is only known at runtime (e.g. user selects metric via config)
- You need a heterogeneous collection of different types

```rust
// ✅ Runtime polymorphism: different metric types in one Vec
fn run_all_metrics(
    metrics: &[Box<dyn Metric>],
    points: &[OperationPoint],
) -> Vec<f64> {
    metrics.iter().map(|m| m.compute(points)).collect()
}
```

**Performance rule of thumb:** prefer generics on hot paths (indicator loops, validation chains). Use `dyn Trait` at the boundary layer (HTTP handlers, config-driven dispatch) where the overhead is negligible.

### Trait bounds with `where` for readability

When a function has multiple bounds, use `where` — it keeps the signature readable.

```rust
// ❌ Cramped
fn process<R: OperationPointRepository + Send + Sync>(repo: &R) { ... }

// ✅ Clean
fn process<R>(repo: &R)
where
    R: OperationPointRepository + Send + Sync,
{ ... }
```

### Supertraits

Use supertraits to express "this trait requires another trait to already be implemented". Don't use them to bundle unrelated behaviour.

```rust
// ✅ A repository that can also be inspected for diagnostics
pub trait DiagnosticRepository: OperationPointRepository {
    fn connection_count(&self) -> usize;
}
```

### Default implementations

Provide sensible defaults for methods that most implementors won't need to override. This reduces boilerplate without sacrificing flexibility.

```rust
pub trait Metric {
    fn compute(&self, points: &[OperationPoint]) -> f64;

    /// Returns the metric name for logging/reporting.
    /// Override if the type name is not descriptive enough.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}
```

---

## 2. Generics & Type Parameters

### Generic structs

Use generic structs when the same structure needs to hold different payload types without duplicating code.

```rust
/// A backtesting command carrying an arbitrary configuration payload.
/// `C` is the config type — e.g. `SharpeConfig`, `DrawdownConfig`.
pub struct BacktestCommand<C> {
    pub instrument: String,
    pub config: C,
}

impl<C> BacktestCommand<C> {
    pub fn new(instrument: impl Into<String>, config: C) -> Self {
        Self { instrument: instrument.into(), config }
    }

    pub fn config(&self) -> &C {
        &self.config
    }
}

// Specialised method only available when C = SharpeConfig
impl BacktestCommand<SharpeConfig> {
    pub fn annualisation_factor(&self) -> u32 {
        self.config.trading_days_per_year
    }
}
```

### Constrain generics only as much as needed

Don't add bounds at the struct definition unless the struct itself needs them. Add bounds at the `impl` block or function where they're actually used.

```rust
// ❌ Forces all users of BacktestCommand to satisfy Serialize even if they don't need it
struct BacktestCommand<C: Serialize> { ... }

// ✅ Bound only where serialisation is actually required
impl<C: Serialize> BacktestCommand<C> {
    pub fn to_json(&self) -> String { ... }
}
```

---

## 3. Error Handling

### The cardinal rule: never `.unwrap()` in production code

`.unwrap()` panics on `None`/`Err`. A panic in an async Tokio task crashes that task silently. In a backtesting run processing thousands of operation points, one bad parse kills the whole run.

```rust
// ❌ Will panic if the map doesn't contain the key
let card = credit_cards.get(name).unwrap();

// ✅ Propagate the error to the caller
let card = credit_cards.get(name)
    .ok_or(LookupError::NotFound(name.to_owned()))?;
```

The only acceptable uses of `.unwrap()` are:
- Inside `#[test]` functions
- On values that are **provably** non-None/non-Err (document why inline)

### Define typed domain errors with enums

`Box<dyn Error>` is fine for quick scripts. In production, typed errors let callers match on variants and handle each case explicitly.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("operation point slice is empty")]
    Empty,

    #[error("mixed indicator types: found {found}, expected {expected}")]
    MixedIndicatorTypes { found: String, expected: String },

    #[error("mixed instruments: found {found}, expected {expected}")]
    MixedInstruments { found: String, expected: String },
}
```

> **Crate recommendation:** use [`thiserror`](https://docs.rs/thiserror) for library/domain errors (gives you `Display` + `Error` for free). Use [`anyhow`](https://docs.rs/anyhow) only in binary entry points (`main`, integration test harnesses) where you just want to propagate and display.

> **Recorded deviation:** repository ports in `domain` return `anyhow::Result`. See the decisions-log entry "anyhow at Domain Ports" for the rationale and revisit triggers.

### Propagate with `?`, convert with `map_err`

The `?` operator propagates errors up the call stack. When the error types don't match, use `.map_err` to convert.

```rust
// ✅ Clean propagation — each ? converts and propagates
fn load_operation_points(path: &str) -> Result<Vec<OperationPoint>, LoadError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| LoadError::Io(e))?;

    let points = serde_json::from_str(&raw)
        .map_err(|e| LoadError::Parse(e))?;

    Ok(points)
}
```

### Use `?` with `Option` too

`?` works on `Option` inside functions returning `Option`. This eliminates nested `if let` chains.

```rust
// ❌ Nested and noisy
fn get_initials(user: &User) -> Option<String> {
    let first = user.firstname.chars().next();
    if let Some(f) = first {
        let last = user.lastname.chars().next();
        if let Some(l) = last {
            return Some(format!("{f}.{l}."));
        }
    }
    None
}

// ✅ Flat and readable
fn get_initials(user: &User) -> Option<String> {
    let f = user.firstname.chars().next()?;
    let l = user.lastname.chars().next()?;
    Some(format!("{f}.{l}."))
}
```

### Match on error variants at the boundary

Handle errors at the layer that knows what to do with them. Domain functions return `Result`; HTTP handlers match on variants and map to status codes.

```rust
// ✅ Handler matches on typed error — each variant maps to a specific HTTP response
async fn validate_handler(
    points: Vec<OperationPoint>,
) -> impl IntoResponse {
    match validate_operation_points(&points) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(ValidationError::Empty) => {
            (StatusCode::BAD_REQUEST, "no operation points provided").into_response()
        }
        Err(ValidationError::MixedInstruments { found, expected }) => {
            (StatusCode::UNPROCESSABLE_ENTITY,
             format!("instrument mismatch: got {found}, expected {expected}"))
                .into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
```

---

## 4. Functional Patterns & Iterators

### Prefer iterator chains over imperative loops

Iterator chains are lazy (no intermediate allocations until `.collect()`), composable, and express intent clearly. The compiler optimises them as well as hand-written loops.

```rust
// ❌ Imperative: mutable state, nested ifs, hard to read
let mut profitable_points = vec![];
for point in &operation_points {
    if let Some(pnl) = point.realised_pnl {
        if pnl > 0.0 {
            profitable_points.push(point.clone());
        }
    }
}

// ✅ Declarative: no mutation, reads like a sentence
let profitable_points: Vec<&OperationPoint> = operation_points
    .iter()
    .filter(|p| p.realised_pnl.map_or(false, |pnl| pnl > 0.0))
    .collect();
```

### `filter_map` for combined filter + transform

When you need to both filter and transform in one pass, `filter_map` is cleaner than `filter` + `map`.

```rust
// ✅ Parse raw strings into domain structs, silently skipping malformed rows
let valid_points: Vec<OperationPoint> = raw_rows
    .iter()
    .filter_map(|row| OperationPoint::try_from(row.as_str()).ok())
    .collect();
```

### Collect into `Result` to fail fast

When parsing a collection where any failure should abort the whole operation, collect into `Result<Vec<_>, _>`. This short-circuits on the first error.

```rust
// ✅ Fails on first parse error — no partial results
let timestamps: Result<Vec<i64>, _> = raw_values
    .iter()
    .map(|s| s.parse::<i64>())
    .collect();

let timestamps = timestamps.map_err(|e| ParseError::InvalidTimestamp(e))?;
```

### Iteration ownership: `iter()` vs `iter_mut()` vs `into_iter()`

| Method | Yields | Ownership |
|---|---|---|
| `.iter()` | `&T` | Borrows, collection still usable |
| `.iter_mut()` | `&mut T` | Mutably borrows, can modify in place |
| `.into_iter()` | `T` | Consumes the collection |

```rust
// Read-only scan — borrow
for point in &operation_points { ... }

// Mutate in place — mutable borrow
for point in &mut operation_points { point.normalise(); }

// Consume and transform — ownership transfer
let pnls: Vec<f64> = operation_points.into_iter().map(|p| p.pnl).collect();
// operation_points is moved — can't use it after this
```

### Closures: capture by reference by default, `move` when crossing thread boundaries

```rust
let threshold = 0.05;

// ✅ Borrows threshold — fine for same-thread use
let above_threshold: Vec<_> = points.iter()
    .filter(|p| p.pnl > threshold)
    .collect();

// ✅ move closure — required when spawning tasks (threshold must be owned by the task)
tokio::spawn(async move {
    let above = points.iter().filter(|p| p.pnl > threshold).count();
    println!("above threshold: {above}");
});
```

---

## 5. Memory Management & Smart Pointers

### Default to stack allocation

Most structs in this codebase should live on the stack. Only reach for heap allocation when you have a concrete reason.

### `Box<T>`: single owner, heap allocation

Use `Box<T>` when:
- The type is recursive (a struct containing itself — the compiler needs a known size)
- You need to erase a concrete type behind a trait object (`Box<dyn Trait>`)
- You're returning a trait object from a function

```rust
// ✅ Recursive type — Box breaks the infinite size cycle
pub struct IndicatorNode {
    pub value: f64,
    pub next: Option<Box<IndicatorNode>>,
}

// ✅ Heterogeneous collection of validators behind a trait object
pub struct ValidationPipeline {
    validators: Vec<Box<dyn Validator>>,
}
```

### `Arc<T>`: shared ownership across threads

Use `Arc<T>` (atomic reference count) when multiple async tasks or threads need to share read access to the same data. This is the correct type for injecting a repository or config into Tokio tasks.

```rust
// ✅ Repository shared across concurrent request handlers
#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn OperationPointRepository + Send + Sync>,
}

// Each handler clone is cheap — just increments the reference count
async fn get_points(State(state): State<AppState>) -> impl IntoResponse {
    let points = state.repo.find_by_instrument("BTCUSDT");
    Json(points)
}
```

**Never use `Rc<T>` in async code.** `Rc` is not `Send` — it cannot cross thread boundaries. Use `Arc` instead.

```rust
// ❌ Rc is single-threaded only — will not compile if sent across threads
use std::rc::Rc;
let db = Rc::new(Database::new());

// ✅ Arc works across threads
use std::sync::Arc;
let db = Arc::new(Database::new());
```

### `Mutex<T>` / `RwLock<T>`: interior mutability across threads

When shared state needs to be mutated, wrap it in `Arc<Mutex<T>>`. For read-heavy workloads (many readers, rare writes), prefer `Arc<RwLock<T>>`.

```rust
// ✅ Shared mutable cache — Arc for shared ownership, Mutex for safe mutation
pub struct MetricsCache {
    inner: Arc<Mutex<HashMap<String, f64>>>,
}

impl MetricsCache {
    pub fn insert(&self, key: String, value: f64) {
        // Lock is held only for the duration of this block
        let mut cache = self.inner.lock().expect("mutex poisoned");
        cache.insert(key, value);
    }
}
```

> **Mutex poisoning:** `.lock().unwrap()` panics if the mutex is poisoned (a thread panicked while holding the lock). In production, handle this explicitly or use `.lock().expect("mutex poisoned — this is a bug")` with a clear message.

---

## 6. Lifetimes

### Let the compiler elide lifetimes where possible

Rust's lifetime elision rules handle the common cases. Only annotate explicitly when the compiler asks you to or when the relationship between input and output lifetimes is non-obvious.

```rust
// ✅ Elision handles this — no annotation needed
fn first_instrument(points: &[OperationPoint]) -> &str {
    &points[0].instrument
}
```

### Annotate when the output lifetime depends on one of multiple inputs

When a function takes multiple references and returns a reference, the compiler can't infer which input the output borrows from. You must be explicit.

```rust
// ❌ Won't compile — compiler doesn't know if result borrows from p1 or p2
fn longer_name(p1: &str, p2: &str) -> &str {
    if p1.len() >= p2.len() { p1 } else { p2 }
}

// ✅ Lifetime 'a ties the output to both inputs — result lives as long as the shorter
fn longer_name<'a>(p1: &'a str, p2: &'a str) -> &'a str {
    if p1.len() >= p2.len() { p1 } else { p2 }
}
```

### Structs holding references need lifetime annotations

If a struct holds a reference, it must declare a lifetime parameter. This ensures the struct cannot outlive the data it borrows.

```rust
// ✅ BacktestView borrows from the original points slice — cannot outlive it
pub struct BacktestView<'a> {
    pub points: &'a [OperationPoint],
    pub instrument: &'a str,
}
```

### Prefer owned data in long-lived structs

Lifetime annotations in structs add complexity. For structs that live across async `.await` points or are stored in application state, prefer owned data (`String` over `&str`, `Vec<T>` over `&[T]`).

```rust
// ❌ Lifetime annotation required, can't be stored in AppState easily
pub struct Config<'a> {
    pub instrument: &'a str,
}

// ✅ Owned — no lifetime, can be cloned and stored anywhere
pub struct Config {
    pub instrument: String,
}
```

---

## 7. Async & Concurrency

### Always use Tokio as the async runtime

```toml
# Cargo.toml
tokio = { version = "1", features = ["full"] }
```

```rust
// ✅ Entry point — Tokio multi-thread runtime (default)
#[tokio::main]
async fn main() {
    // ...
}
```

### `async fn` is lazy — futures do nothing until awaited

```rust
// ❌ This does NOT execute my_function — it just creates a future
let f = my_function();

// ✅ This executes it
let f = my_function();
f.await;

// Or inline
my_function().await;
```

### Spawn concurrent tasks with `tokio::spawn`

Use `tokio::spawn` to run independent work concurrently. Each spawned task runs on the Tokio thread pool.

```rust
// ✅ Run two database reads concurrently instead of sequentially
async fn load_backtest_data(instrument: &str) -> BacktestData {
    let points_handle = tokio::spawn(fetch_operation_points(instrument.to_owned()));
    let indicators_handle = tokio::spawn(fetch_indicators(instrument.to_owned()));

    // Both fetches run in parallel — total time ≈ max(t1, t2) not t1 + t2
    let (points, indicators) = tokio::join!(points_handle, indicators_handle);

    BacktestData {
        points: points.expect("points task panicked"),
        indicators: indicators.expect("indicators task panicked"),
    }
}
```

### Never block the async runtime with CPU-heavy work

Tokio's async tasks run on a shared thread pool. A blocking computation (tight loop, heavy calculation) starves other tasks. Offload to a dedicated blocking thread pool with `spawn_blocking`.

```rust
// ❌ Blocks the Tokio runtime — all other tasks stall while this runs
async fn compute_sharpe(points: Vec<OperationPoint>) -> f64 {
    // 400 million iterations — this will block for seconds
    heavy_sharpe_calculation(&points)
}

// ✅ Offloads to a blocking thread — Tokio runtime stays responsive
async fn compute_sharpe(points: Vec<OperationPoint>) -> f64 {
    tokio::task::spawn_blocking(move || {
        heavy_sharpe_calculation(&points)
    })
    .await
    .expect("blocking task panicked")
}
```

> **Rule:** if a synchronous function takes more than ~100µs, use `spawn_blocking`. Backtesting metric computation over large datasets is a prime candidate.

### Shared mutable state across tasks: `Arc<Mutex<T>>`

```rust
// ✅ Shared results map — multiple tasks write, main task reads at the end
let results: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(HashMap::new()));

let mut handles = vec![];
for metric in metrics {
    let results = Arc::clone(&results);
    let points = points.clone();
    handles.push(tokio::spawn(async move {
        let value = metric.compute(&points);
        results.lock().unwrap().insert(metric.name().to_owned(), value);
    }));
}

for handle in handles {
    handle.await.unwrap();
}
```

### `tokio::sync::Mutex` vs `std::sync::Mutex`

Use `tokio::sync::Mutex` when the lock must be held across an `.await` point. Use `std::sync::Mutex` (faster) when the lock is acquired and released within a single synchronous block.

```rust
// ✅ std Mutex — lock acquired and released synchronously, no await inside
{
    let mut cache = std::sync::Mutex::lock(&self.cache).unwrap();
    cache.insert(key, value);
} // lock released here

// ✅ tokio Mutex — lock held across an await point
let mut conn = tokio::sync::Mutex::lock(&self.connection).await;
conn.execute(query).await?; // await while holding the lock
```

---

## 8. Architecture: Ports & Adapters

This project uses ports-and-adapters (hexagonal architecture). The domain layer is pure Rust with no I/O dependencies. Repositories are traits (ports); concrete implementations (Postgres, in-memory) are adapters.

### Define ports as traits in the domain layer

```rust
// src/domain/ports.rs — no I/O, no framework imports

/// Port: read/write access to operation points.
#[async_trait::async_trait]
pub trait OperationPointRepository: Send + Sync {
    async fn find_by_instrument(
        &self,
        instrument: &str,
    ) -> Result<Vec<OperationPoint>, RepositoryError>;

    async fn save(
        &self,
        point: OperationPoint,
    ) -> Result<(), RepositoryError>;
}
```

### Inject dependencies via trait objects in `Arc`

```rust
// ✅ AppState holds the port — concrete type is injected at startup
pub struct AppState {
    pub operation_point_repo: Arc<dyn OperationPointRepository>,
}

// In main.rs — wire up the concrete adapter
let pg_repo = Arc::new(PostgresOperationPointRepository::new(pool));
let state = AppState { operation_point_repo: pg_repo };
```

### Use in-memory adapters for tests

Because the domain depends on traits, not concrete types, tests inject a fast in-memory implementation — no database required.

```rust
// tests/common/fixtures.rs
pub struct InMemoryOperationPointRepository {
    points: Mutex<Vec<OperationPoint>>,
}

#[async_trait::async_trait]
impl OperationPointRepository for InMemoryOperationPointRepository {
    async fn find_by_instrument(&self, instrument: &str) -> Result<Vec<OperationPoint>, RepositoryError> {
        let points = self.points.lock().unwrap();
        Ok(points.iter().filter(|p| p.instrument == instrument).cloned().collect())
    }

    async fn save(&self, point: OperationPoint) -> Result<(), RepositoryError> {
        self.points.lock().unwrap().push(point);
        Ok(())
    }
}
```

### Keep domain functions pure

Validation, metric computation, and business rules should be pure functions — no I/O, no side effects, deterministic output. This makes them trivially testable.

```rust
// ✅ Pure function — takes data, returns Result, no I/O
pub fn validate_operation_points(
    points: &[OperationPoint],
) -> Result<(), ValidationError> {
    if points.is_empty() {
        return Err(ValidationError::Empty);
    }

    let first_instrument = &points[0].instrument;
    if points.iter().any(|p| &p.instrument != first_instrument) {
        return Err(ValidationError::MixedInstruments { ... });
    }

    Ok(())
}
```

---

## 9. Anti-Patterns

### ❌ `.unwrap()` / `.expect()` in non-test code

Panics crash async tasks silently. Always propagate with `?` or handle explicitly.

```rust
// ❌
let value = map.get("key").unwrap();

// ✅
let value = map.get("key").ok_or(MyError::KeyNotFound)?;
```

### ❌ `Box<dyn Error>` as a return type in library code

Erases type information — callers can't match on variants. Use typed enums.

```rust
// ❌ Caller has no idea what went wrong
fn parse_file(path: &str) -> Result<Data, Box<dyn std::error::Error>> { ... }

// ✅ Caller can handle each case
fn parse_file(path: &str) -> Result<Data, ParseError> { ... }
```

### ❌ Blocking I/O inside `async fn`

Stalls the entire Tokio runtime. Use `spawn_blocking` for any synchronous blocking work.

```rust
// ❌ std::fs::read_to_string blocks the thread
async fn load(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

// ✅ tokio::fs is async
async fn load(path: &str) -> Result<String, io::Error> {
    tokio::fs::read_to_string(path).await
}
```

### ❌ `Rc<T>` in async code

`Rc` is not `Send`. It will not compile when used across `.await` points or in `tokio::spawn`. Use `Arc`.

### ❌ Global mutable state (`static mut`, `lazy_static` with `Mutex`)

Impossible to test in isolation, creates hidden coupling. Inject state via function parameters or `AppState`.

### ❌ Cloning to avoid borrow checker fights

Cloning is sometimes correct, but cloning large `Vec`s to avoid thinking about lifetimes is a performance smell. Understand the borrow, or restructure the code.

```rust
// ❌ Clones the entire points vec just to pass it to a function
let result = compute_metric(points.clone());

// ✅ Pass a reference — no allocation
let result = compute_metric(&points);
```

### ❌ Ignoring `clippy` warnings

Clippy catches real bugs and anti-patterns. All warnings are treated as errors in this project.

```bash
cargo clippy -- -D warnings   # must pass with zero output
cargo fmt --check             # must pass with zero diff
```

---

## 10. Tooling Checklist

Every task is complete only when all of these pass:

```bash
# Format check — zero diff
cargo fmt --check

# Lint — zero warnings (warnings are errors)
cargo clippy -- -D warnings

# Tests — all pass
cargo test

# Build — clean
cargo build
```

### Recommended crates for this project

| Purpose | Crate | Notes |
|---|---|---|
| Async runtime | `tokio` | Use `features = ["full"]` |
| Domain errors | `thiserror` | Derive `Error` + `Display` on enums |
| Application errors | `anyhow` | Only in `main` / integration tests |
| Serialisation | `serde`, `serde_json` | Derive `Serialize`/`Deserialize` |
| HTTP framework | `axum` | Integrates natively with Tokio |
| Database | `sqlx` | Async, compile-time query checking |
| Trait objects in async | `async-trait` | Until async fn in traits is stable |
| Decimal arithmetic | `rust_decimal` | Never use `f64` for financial values |
| Iterator composition | `itertools` | `all_equal`, `tuple_windows` + `partition_map`, `iproduct!` — extends the §4 iterator-chain house style (adopted 2026-09-17, see decisions-log) |

> **Financial values:** use `rust_decimal::Decimal` for PnL, prices, and percentages. Floating-point arithmetic accumulates rounding errors that compound across thousands of operation points.

