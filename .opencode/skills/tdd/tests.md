# Good and Bad Tests

## Good Tests

**Integration-style**: Test through real interfaces, not mocks of internal parts.

```rust
// GOOD: Tests observable behavior through the public use case interface
#[test]
fn test_backtest_returns_zero_return_when_no_price_change() {
    let repo = FakeBacktestRepository::with_prices(vec![
        PricePoint { date: "2023-01-01".into(), close: 100.0 },
        PricePoint { date: "2023-12-31".into(), close: 100.0 },
    ]);
    let use_case = RunBacktest::new(repo);

    let result = use_case.execute(BacktestParams::default()).unwrap();

    assert_eq!(result.total_return, 0.0);
}
```

Characteristics:

- Tests behavior callers care about
- Uses public API only (`RunBacktest::execute`)
- Survives internal refactors (could swap calculator internals — test doesn't care)
- Describes WHAT, not HOW
- One logical assertion per test

## Bad Tests

**Implementation-detail tests**: Coupled to internal structure.

```rust
// BAD: Tests that ReturnCalculator was called — not that the result is correct
#[test]
fn test_run_backtest_calls_calculator() {
    // Mocking internal collaborators couples the test to implementation details.
    // If ReturnCalculator is refactored or inlined, this test breaks
    // even though behavior is unchanged.
    let mock_calc = MockReturnCalculator::new();
    mock_calc.expect_calculate().return_once(|| 0.42);

    let use_case = RunBacktest::with_calculator(mock_calc);
    use_case.execute(BacktestParams::default()).unwrap();

    mock_calc.assert_called_once(); // asserting HOW, not WHAT
}
```

Red flags:

- Mocking internal collaborators (`ReturnCalculator` is an implementation detail)
- Test breaks when refactoring without behavior change
- Test name describes HOW, not WHAT
- Asserting on call counts/order instead of return values

---

**DB bypass**: Querying the database directly instead of through the interface.

```rust
// BAD: Bypasses BacktestRepository to verify storage
#[test]
fn test_backtest_saved(pool: &DbPool) {
    insert_test_data(&pool, &[BacktestFixture::default()]);

    let row = pool.query_one(
        "SELECT id FROM backtests WHERE symbol = $1",
        &["AAPL"]
    ).unwrap();

    assert!(row.get::<_, i64>(0) > 0); // Verifying through DB, not interface
}


// GOOD: Verifies through the repository interface
#[test]
fn test_backtest_is_retrievable() {
    let repo = DbBacktestRepository::new(test_pool());
    let backtests = repo.list_by_symbol("AAPL").unwrap();

    assert_eq!(backtests.len(), 1);
    assert_eq!(backtests[0].symbol, "AAPL");
}
```
