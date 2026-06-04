<!-- Context: project-intelligence/business | Priority: high | Version: 1.1 | Updated: 2026-06-04 -->

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

The engine is in **early development**:
- ✅ Domain model defined (entities, repository traits)
- ✅ Five validation rules implemented and tested
- ✅ In-memory repository adapters for testing
- ✅ CLI structure (`bt` binary with dev, indicator, mms, operation-point, raw-point, redis, resampled-point, strategy commands)
- ✅ Engine CLI module extracted (`bt-engine` binary with its own command tree)
- ✅ Environment/DB configuration
- ⬜ Backtest execution logic (not yet implemented)
- ⬜ Performance metrics calculation
- ⬜ Persistent storage adapters (PostgreSQL)
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
