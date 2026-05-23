<!-- Context: development/backend/rust/testing | Priority: high | Version: 1.0 | Updated: 2026-05-23 -->

# Rust Testing Patterns

**Stack**: cargo test · actix-web test utilities · Rust 2021 edition

---

## Test Structure

```
src/
├── lib.rs
├── main.rs
└── backtests/
    ├── mod.rs
    └── tests.rs        # or inline #[cfg(test)] modules

tests/                  # integration tests (separate crate)
└── integration_test.rs
```

Place unit tests in the same file as the code under test using `#[cfg(test)]`. Place integration tests in `tests/`.

---

## Unit Test Pattern

```rust
// src/calculator.rs
pub fn calculate_return(start: f64, end: f64) -> f64 {
    (end - start) / start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_return() {
        let result = calculate_return(100.0, 110.0);
        assert!((result - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_negative_return() {
        let result = calculate_return(100.0, 90.0);
        assert!((result - (-0.1)).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic]
    fn test_zero_start_panics() {
        calculate_return(0.0, 100.0);
    }
}
```

---

## Async Tests

Use `#[actix_web::test]` for async handler tests:

```rust
#[cfg(test)]
mod tests {
    use actix_web::{test, web, App};
    use super::*;

    #[actix_web::test]
    async fn test_get_backtest_returns_200() {
        let app = test::init_service(
            App::new().route("/backtests/{id}", web::get().to(get_backtest))
        ).await;

        let req = test::TestRequest::get()
            .uri("/backtests/1")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
```

---

## Fixtures / Test Helpers

Use helper functions or builder patterns instead of fixtures:

```rust
#[cfg(test)]
mod tests {
    fn sample_backtest() -> Backtest {
        Backtest {
            id: 1,
            name: "Test Backtest".to_string(),
            start_date: "2023-01-01".to_string(),
        }
    }

    fn empty_repository() -> FakeBacktestRepository {
        FakeBacktestRepository::new(vec![])
    }
}
```

---

## Mocking / Fakes

Use trait objects and fake implementations — no extra mock library needed:

```rust
// domain/repositories.rs — the contract
pub trait BacktestRepository: Send + Sync {
    fn list(&self) -> Vec<Backtest>;
    fn find(&self, id: u64) -> Option<Backtest>;
}

// testing fakes
pub struct FakeBacktestRepository {
    data: Vec<Backtest>,
}

impl FakeBacktestRepository {
    pub fn new(data: Vec<Backtest>) -> Self { Self { data } }
}

impl BacktestRepository for FakeBacktestRepository {
    fn list(&self) -> Vec<Backtest> { self.data.clone() }
    fn find(&self, id: u64) -> Option<Backtest> {
        self.data.iter().find(|b| b.id == id).cloned()
    }
}
```

---

## Running Tests

```bash
# Run all tests
cargo test

# Run with output (show println!)
cargo test -- --nocapture

# Run a specific test
cargo test test_positive_return

# Run tests in a specific module
cargo test backtests::tests

# Run integration tests only
cargo test --test integration_test

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out term-missing
```

---

## Coverage Config

Install `cargo-tarpaulin` for coverage:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --exclude-files "src/main.rs" --out term-missing
```

---

## Linting in Tests (clippy)

Clippy rules are relaxed for tests via `#[allow(...)]` when needed:

```rust
#[cfg(test)]
mod tests {
    #[allow(clippy::float_cmp)]
    #[test]
    fn test_exact_float() { ... }
}
```

---

## Related Context

- **Actix-web patterns** → `actix-patterns.md`
- **Cargo commands** → `cargo.md`
