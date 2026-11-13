<!-- Context: project-intelligence/business | Priority: high | Version: 1.10 | Updated: 2026-11-13 -->

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
          and indicator configurations from PostgreSQL, validates data consistency,
          computes real indicator signals, and evaluates strategies with performance
          metrics — persisting survivors to partitioned Parquet output.
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
- Real indicator mathematics (14 indicators) with explicit signal semantics — no black box
- Idempotent batch runs — re-running never duplicates work or output

**For Business**:
- Foundation for a full backtesting platform (metrics, reporting, optimization)
- Clean architecture enables adding new data sources, indicators, and strategies without touching core logic
- Parquet output plugs directly into the analytical ecosystem (Arrow/Polars/Spark)

## Current State

The engine is in **active development** — the strategy pipeline is real end-to-end:
- ✅ Domain model defined (entities, repository traits)
- ✅ Five validation rules implemented and tested (fail-fast, ordered)
- ✅ Engine CLI (`bte` binary: strategy create, create_loop)
- ✅ Environment/DB configuration (production / development / testing)
- ✅ PostgreSQL adapters for indicators, operation points, resampled points (+ indexed batch loads)
- ✅ `Strategy` entity + `GlobalStrategyFilter` with `admits_activity()` / `admits_performance()` (DD ≤ 20%, ROI ≥ 5%, ops ≥ 10/yr on matched trades)
- ✅ Application layer (`fetch_source_data`, `create_signal_groups`, `create_strategies`, `run_strategies`)
- ✅ Performance metrics: single-pass `performance_metrics()` (global ROI + max draw down), `annual_roi()`, `annual_operation_count()` — pure domain functions
- ✅ Real metrics computed in the strategy generator (rayon-parallelized)
- ✅ **Real signal generation** — 14 indicators computed from candle data (RSI, AO, AC, Bears, Bulls, CHO, CHV, DPO, Force, OsMa, MACD, ROC, TRIX, VROC) with zero-cross / threshold / crossover signal semantics
- ✅ Two-stage filtering: signal groups by annual op count, then strategies by DD + ROI
- ✅ In-memory signal-group memoization (`cached` crate, keyed by `indicator_id` + instrument)
- ✅ `SignalGroup` in domain layer with embedded `indicator_id`, `instrument`, `operation_count`
- ✅ All timestamps are `i64` (epoch seconds)
- ✅ Partitioned Parquet persistence (instrument × MMS, ZSTD, idempotent by indicator_id)
- ✅ Processed manifest — idempotency ledger of (instrument, mms_id, indicator_id) triples
- ✅ `create_loop` — batch pipeline over all cells, parallel compute + persist
- ✅ CI: fmt + clippy (-D warnings) + tests + cargo-deny + Docker build/push
- ✅ MACD signal generation (MACD line crossing its signal line; signal period fixed at 9)
- ⬜ Exit-date-aware metrics (ROI/DD currently assume operations close in entry order — see living-notes)
- ⬜ API endpoints for running backtests (deliberately deferred — no consumers yet)

## Business Constraints

- **Single-instrument runs**: A backtest evaluates one instrument at a time
- **Single-MMS runs**: A backtest evaluates one money management strategy at a time
- **Timestamp alignment**: Trades must map to existing market data points

## Related Files

- `domain-model.md` — Full entity definitions, glossary, and validation rules
- `business-tech-bridge.md` — How domain concepts map to code
- `decisions-log.md` — Architectural decisions with context
