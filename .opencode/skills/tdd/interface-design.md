# Interface Design for Testability

Good interfaces make testing natural:

**1. Accept dependencies, don't create them**

```rust
// GOOD: testable — repository injected, can be swapped for a fake
pub struct RunBacktest<R: BacktestRepository> {
    repo: R,
}

impl<R: BacktestRepository> RunBacktest<R> {
    pub fn new(repo: R) -> Self { Self { repo } }

    pub fn execute(&self, params: BacktestParams) -> Result<BacktestResult, AppError> {
        let prices = self.repo.load_prices(&params.symbol)?;
        // ...
    }
}


// BAD: hard to test — creates its own DB connection internally
pub struct RunBacktest;

impl RunBacktest {
    pub fn execute(&self, params: BacktestParams) -> Result<BacktestResult, AppError> {
        let prices = DbBacktestRepository::new(get_conn()).load_prices(&params.symbol)?;
        // ...
    }
}
```

**2. Return results, don't produce side effects**

```rust
// GOOD: returns a value — easy to assert on
pub fn calculate_return(&self, prices: &[PricePoint]) -> f64 {
    let start = prices.first().map(|p| p.close).unwrap_or(0.0);
    let end = prices.last().map(|p| p.close).unwrap_or(0.0);
    (end - start) / start
}


// BAD: mutates shared state — requires inspecting the struct after the call
pub fn calculate_return(&mut self, prices: &[PricePoint]) {
    let start = prices.first().map(|p| p.close).unwrap_or(0.0);
    let end = prices.last().map(|p| p.close).unwrap_or(0.0);
    self.result = (end - start) / start; // side effect
}
```

This is why `ReturnCalculator::calculate()` returns `f64` directly, and `RunBacktest::execute()` returns `Result<BacktestResult, AppError>` — the caller asserts the return value, not a side effect.

**3. Small surface area**

Fewer methods = fewer tests needed. Fewer params = simpler test setup.

```rust
// GOOD: one method, one clear contract
pub trait BacktestRepository: Send + Sync {
    fn load_prices(&self, symbol: &str) -> Result<Vec<PricePoint>, AppError>;
}


// BAD: wide surface — every method needs its own test setup and fake implementation
pub trait BacktestRepository: Send + Sync {
    fn get_by_id(&self, id: u64) -> Result<Option<Backtest>, AppError>;
    fn get_by_symbol(&self, symbol: &str) -> Result<Vec<Backtest>, AppError>;
    fn get_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Backtest>, AppError>;
    fn get_by_strategy(&self, strategy: &str) -> Result<Vec<Backtest>, AppError>;
    fn count(&self) -> Result<usize, AppError>;
}
```

Filtering is pushed inside via a `BacktestQuery` struct, keeping the interface to one method while hiding all the filtering complexity inside the implementation.
