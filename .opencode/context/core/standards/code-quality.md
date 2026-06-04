<!-- Context: standards/code | Priority: critical | Version: 2.2 | Updated: 2026-05-23 -->
# Code Standards

## Quick Reference

> 🚨 **MANDATORY**: All code written or modified must pass `cargo clippy -- -D warnings` and
> `cargo fmt --check` with **zero errors** before the task is considered complete.
> Write clippy-compliant code from the start — do not wait for clippy to report violations.

**Core Philosophy**: Modular, Functional, Maintainable
**Golden Rule**: If you can't easily test it, refactor it

**Critical Patterns** (use these):
- ✅ Pure functions (same input = same output, no side effects)
- ✅ Immutability (prefer `let` over `let mut`, use owned types)
- ✅ Composition (build complex from simple)
- ✅ Small functions (< 50 lines)
- ✅ Explicit dependencies (dependency injection via traits)

**Anti-Patterns** (avoid these):
- ❌ Mutation, side effects, deep nesting
- ❌ God modules, global state, large functions

---

## Core Philosophy

**Modular**: Everything is a component - small, focused, reusable
**Functional**: Pure functions, immutability, composition over inheritance
**Maintainable**: Self-documenting, testable, predictable

## Principles

### Modular Design
- Single responsibility per module
- Clear interfaces (explicit inputs/outputs via traits)
- Independent and composable
- < 100 lines per component (ideally < 50)

### Functional Approach
- **Pure functions**: Same input = same output, no side effects
- **Immutability**: Prefer `let` bindings; use `mut` only when necessary
- **Composition**: Build complex from simple functions
- **Declarative**: Describe what, not how

### Component Structure
```
src/
├── lib.rs          # Public interface
├── core.rs         # Core logic (pure functions)
├── utils.rs        # Helpers
└── tests/          # Integration tests
```

## Patterns

### Pure Functions
```rust
// ✅ Pure
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn format_user(first: &str, last: &str) -> String {
    format!("{} {}", first, last)
}

// ❌ Impure (side effects)
static mut TOTAL: i32 = 0;
fn add_to_total(value: i32) -> i32 {
    unsafe { TOTAL += value; TOTAL }
}
```

### Immutability
```rust
// ✅ Immutable — return new value
fn add_item(mut items: Vec<String>, item: String) -> Vec<String> {
    items.push(item);
    items
}

// ✅ Prefer owned/cloned over mutation of shared state
let updated = User { name: "Alice".to_string(), ..user };

// ❌ Mutating shared state
fn add_item(items: &mut Vec<String>, item: String) {
    items.push(item); // mutates caller's data
}
```

### Composition
```rust
// ✅ Compose small functions
fn process_user(user: User) -> Result<User, AppError> {
    validate_user(user)
        .and_then(enrich_user_data)
        .and_then(save_user)
}

fn is_valid_email(email: &str) -> bool {
    validate_email(normalize_email(email))
}

// ❌ Deep inheritance hierarchies — use traits instead
```

### Declarative
```rust
// ✅ Declarative
let active_users: Vec<&str> = users.iter()
    .filter(|u| u.is_active)
    .map(|u| u.name.as_str())
    .collect();

// ❌ Imperative
let mut names = vec![];
for user in &users {
    if user.is_active {
        names.push(user.name.as_str());
    }
}
```

## Naming

- **Files/modules**: `snake_case.rs`
- **Functions**: `verb_phrases` (`get_user`, `validate_email`)
- **Predicates**: `is_valid`, `has_permission`, `can_access`
- **Variables**: descriptive (`user_count` not `uc`)
- **Constants**: `UPPER_SNAKE_CASE`
- **Types/Structs/Enums**: `PascalCase`
- **Traits**: `PascalCase` (e.g. `BacktestRepository`)

## Error Handling

```rust
// ✅ Use Result for recoverable errors
use std::num::ParseIntError;

fn parse_id(s: &str) -> Result<u64, ParseIntError> {
    s.parse::<u64>()
}

// ✅ Propagate with ?
fn process(s: &str) -> Result<String, AppError> {
    let id = parse_id(s)?;
    Ok(format!("id={}", id))
}

// ✅ Validate at boundaries
fn create_backtest(name: &str) -> Result<Backtest, AppError> {
    if name.is_empty() {
        return Err(AppError::Validation("name cannot be empty".into()));
    }
    Ok(Backtest { name: name.to_string() })
}
```

## Dependency Injection

```rust
// ✅ Dependencies via traits — easy to swap in tests
pub trait BacktestRepository: Send + Sync {
    fn find(&self, id: u64) -> Option<Backtest>;
}

pub struct BacktestService<R: BacktestRepository> {
    repo: R,
}

impl<R: BacktestRepository> BacktestService<R> {
    pub fn new(repo: R) -> Self { Self { repo } }

    pub fn get(&self, id: u64) -> Result<Backtest, AppError> {
        self.repo.find(id).ok_or(AppError::NotFound(id.to_string()))
    }
}

// ❌ Hidden dependencies — hard to test, hard to swap
fn get_backtest(id: u64) -> Backtest {
    DB_POOL.get().find(id) // global state
}
```

## Anti-Patterns

❌ **Mutation**: Prefer immutable bindings; use `mut` only when necessary
❌ **Side effects**: No I/O inside pure functions
❌ **Deep nesting**: Use early returns and `?` operator instead
❌ **God modules**: Split into focused modules
❌ **Global state**: Pass dependencies explicitly via traits
❌ **Large functions**: Keep < 50 lines
❌ **`unwrap()` in production code**: Use `?` or explicit error handling

## Best Practices

✅ Pure functions whenever possible
✅ Immutable data structures
✅ Small, focused functions (< 50 lines)
✅ Compose small functions into larger ones
✅ Explicit dependencies (dependency injection via traits)
✅ Validate at boundaries
✅ Self-documenting code
✅ Test in isolation

**Golden Rule**: If you can't easily test it, refactor it.

---

## Rust — Linting & Formatting (clippy + rustfmt)

This project uses **clippy** for linting and **rustfmt** for formatting.

### Configuration

- `clippy.toml` or `#[allow(...)]` for per-file overrides
- `rustfmt.toml` for formatting rules (default: standard Rust style)

### Commands

```bash
# Lint (treat warnings as errors)
cargo clippy -- -D warnings

# Lint + auto-fix safe issues
cargo clippy --fix

# Format
cargo fmt

# Check formatting without modifying
cargo fmt --check
```

> ⚠️ Every file generated or modified **must pass `cargo clippy -- -D warnings` with zero errors** before being committed.

### Key rules enforced

| Rule | Description |
|------|-------------|
| `clippy::unwrap_used` | No `.unwrap()` in production — use `?` or `expect` with message |
| `clippy::clone_on_ref_ptr` | Avoid unnecessary clones |
| `clippy::needless_pass_by_value` | Pass by reference when ownership not needed |
| `clippy::match_wildcard_for_single_variants` | Explicit match arms over `_` catch-all |
| `rustfmt` | 4-space indent, 100-char line limit (Rust default) |

### Quick examples

```rust
// ✅ Imports — group by std, external, local
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::Backtest;

// ✅ Error propagation
fn load(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

// ✅ Doc comment
/// Calculates the annualised return for a backtest.
///
/// # Arguments
/// * `start` - Starting portfolio value
/// * `end` - Ending portfolio value
/// * `years` - Duration in years
pub fn annualised_return(start: f64, end: f64, years: f64) -> f64 {
    (end / start).powf(1.0 / years) - 1.0
}
```
