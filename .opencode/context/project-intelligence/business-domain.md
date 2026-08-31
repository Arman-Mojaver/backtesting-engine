<!-- Context: project-intelligence/business | Priority: high | Version: 1.5 | Updated: 2026-08-31 -->

# Business Domain

> Why this project exists and what problem it solves.

## Project Identity

```
Project Name: Backtesting Engine
Tagline: Validate trading strategies against historical market data
Problem Statement: Traders need to evaluate whether a strategy (indicator + money
                   management rules) would have been profitable on past data before
                   risking real capital.
Solution: A Rust engine that loads historical OHLCV candles, long/short trade records,
          and indicator configurations, validates data consistency, and (planned)
          computes performance metrics.
```

## Target Users

| User Segment | Who They Are | What They Need | Pain Points |
|--------------|--------------|----------------|-------------|
| Quantitative traders | Individuals or teams developing systematic strategies | Fast, reliable backtesting with strict data validation | Garbage-in/garbage-out — bad data produces misleading results |
| Strategy developers | Engineers building and iterating on trading algorithms | Programmatic API to plug strategies into a validation + simulation pipeline | Manual spreadsheet backtesting is slow and error-prone |

## Value Proposition

**For Users**:
- Catches data inconsistencies (mixed instruments, misaligned timestamps, wrong MMS) before they corrupt results
- Separates long and short analysis for directional clarity
- Pure-domain validation — no database or network required to verify data integrity

**For Business**:
- Foundation for a full backtesting platform (metrics, reporting, optimization)
- Clean architecture enables adding new data sources, indicators, and strategies without touching core logic

## Current State

The engine is in **active development**:
- ✅ Domain model defined (entities, repository traits)
- ✅ Five validation rules implemented and tested
- ✅ In-memory repository adapters for testing
- ✅ CLI structure (`ct` binary with dev, indicator, mms, operation-point, raw-point, resampled-point, strategy commands)
- ✅ Engine CLI module extracted (`bt-engine` binary with its own command tree)
- ✅ Environment/DB configuration
- ✅ PostgreSQL adapters for indicators, operation points, resampled points
- ✅ `Strategy` entity + `GlobalStrategyFilter` + `filter_strategies()`
- ✅ Strategy generation (`create` from DB, `generate` synthetic) with partitioned parquet persistence
- ✅ Application layer (`fetch_strategy_data`, `create_signals`, `create_strategies`)
- ✅ Performance metric functions: `performance_metrics()` (single-pass global ROI + max draw down, `annual_roi()` method, defensive chronological sort), `annual_operation_count()` (pure domain functions)
- ✅ Real metrics computed in the strategy generator (`create_strategies()` — no more random placeholders)
- ✅ Signal generator (`create_signals()` — random timestamp sampling from resampled points)
- ✅ Two-stage filtering: signal groups by annual op count, then strategies by DD + ROI
- ✅ In-memory signal-group memoization (`cached` crate, keyed by `indicator_id` + instrument)
- ✅ `SignalGroup` in domain layer with embedded `indicator_id`
- ✅ All timestamps are `i64` (epoch seconds)
- ⬜ Backtest execution logic (not yet implemented)
- ⬜ Real signal generation from indicator parameters (MACD crossover, RSI thresholds — currently random timestamps)
- ⬜ API endpoints for running backtests

## Business Constraints

- **Single-instrument runs**: A backtest evaluates one instrument at a time
- **Single-MMS runs**: A backtest evaluates one money management strategy at a time
- **Single-indicator-type runs**: All indicators must be the same type per run
- **Timestamp alignment**: Trades must map to existing market data points

## Related Files

- `domain-model.md` — Full entity definitions, glossary, and validation rules
- `business-tech-bridge.md` — How domain concepts map to code
- `decisions-log.md` — Architectural decisions with context
