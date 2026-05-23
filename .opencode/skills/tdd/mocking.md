# When to Mock

Mock at **system boundaries** only:

- External APIs (payment, email, etc.)
- Databases (use a fake/in-memory implementation — not a mock of the ORM)
- Time / randomness
- File system (sometimes)

Don't mock:

- Your own structs or domain logic
- Internal collaborators
- Anything you own and control

## Designing for Mockability

At system boundaries, design traits that are easy to substitute in tests.

**1. Use dependency injection**

Pass external dependencies in rather than creating them internally:

```rust
// GOOD: dependency injected — easy to substitute in tests
pub struct RunBacktest<R: BacktestRepository> {
    repo: R,
}

impl<R: BacktestRepository> RunBacktest<R> {
    pub fn new(repo: R) -> Self { Self { repo } }

    pub fn execute(&self, params: BacktestParams) -> Result<BacktestResult, AppError> {
        let data = self.repo.load_prices(&params.symbol)?;
        // ...
    }
}


// BAD: dependency hardcoded — impossible to test without a real DB
pub struct RunBacktest;

impl RunBacktest {
    pub fn execute(&self, params: BacktestParams) -> Result<BacktestResult, AppError> {
        let data = DbBacktestRepository::new(get_conn()).load_prices(&params.symbol)?;
        // ...
    }
}
```

In tests, swap in the fake:

```rust
#[test]
fn test_backtest_with_no_data_returns_error() {
    let repo = FakeBacktestRepository::new(vec![]);
    let result = RunBacktest::new(repo).execute(BacktestParams::default());
    assert!(result.is_err());
}
```

**2. Define boundaries with traits**

Use a trait to define the contract. Production code gets the real DB adapter; tests get a fake:

```rust
// src/domain/repositories.rs — the contract (boundary)
pub trait BacktestRepository: Send + Sync {
    fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError>;
}


// src/adapters/db_backtest_repository.rs — real implementation
pub struct DbBacktestRepository { /* db pool */ }

impl BacktestRepository for DbBacktestRepository {
    fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError> {
        // hits DB
        todo!()
    }
}


// src/testing/doubles.rs — fake for tests
pub struct FakeBacktestRepository {
    prices: Vec<PricePoint>,
}

impl FakeBacktestRepository {
    pub fn new(prices: Vec<PricePoint>) -> Self { Self { prices } }
}

impl BacktestRepository for FakeBacktestRepository {
    fn load_prices(&self, _symbol: &str) -> Result<Vec<PricePoint>, AppError> {
        Ok(self.prices.clone())
    }
}
```

**3. Prefer specific interfaces over generic ones**

Create focused methods for each operation instead of one generic fetch wrapper:

```rust
// GOOD: each method is independently substitutable and clearly typed
pub trait BacktestRepository: Send + Sync {
    fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError>;
}


// BAD: generic fetch wrapper — mocking requires conditional logic,
// unclear what shape is returned per call
pub trait Repository: Send + Sync {
    fn fetch(&self, table: &str, filters: &HashMap<String, String>) -> Result<Vec<serde_json::Value>, AppError>;
}
```

The specific approach means:

- Each fake returns one specific type
- No conditional logic in test setup
- Clear which data a test exercises
- Type-safe per operation

## Actix-web: Override Dependencies in Handler Tests

For handler tests, use `web::Data` with a fake injected via `app_data`:

```rust
#[actix_web::test]
async fn test_get_backtest_returns_empty_for_no_data() {
    let fake_repo = web::Data::new(FakeBacktestRepository::new(vec![]));

    let app = test::init_service(
        App::new()
            .app_data(fake_repo)
            .route("/backtests", web::get().to(list_backtests))
    ).await;

    let req = test::TestRequest::get().uri("/backtests").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
}
```

This avoids mocking at the HTTP layer — the real actix-web routing, request parsing, and response serialisation all run; only the DB boundary is substituted.
