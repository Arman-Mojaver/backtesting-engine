<!-- Context: project-intelligence/domain-model | Priority: critical | Version: 1.1 | Updated: 2026-06-04 -->

# Domain Model — Backtesting Engine

> Ubiquitous language, entity definitions, invariants, and validation rules extracted from the Rust codebase.
> This is the single source of truth for how we talk about the domain.

## Ubiquitous Language (Glossary)

| Term | Definition | Rust Type | Module |
|------|-----------|-----------|--------|
| **Instrument** | A tradeable financial pair (e.g. `EURUSD`, `USDCAD`). Identifies what is being traded. | `String` field | `domain::operation_point`, `domain::resampled_point` |
| **Operation Point** | A single trade entry — a point in time where a position was opened. Contains the trade's result, take-profit, stop-loss, risk, and the money management strategy used. | `OperationPoint` | `domain::operation_point` |
| **Long Operation Point** | An operation point representing a **buy** (long) position. Stored in a separate repository from shorts. | `OperationPoint` via `LongOperationPointRepository` | `domain::operation_point` |
| **Short Operation Point** | An operation point representing a **sell** (short) position. Stored in a separate repository from longs. | `OperationPoint` via `ShortOperationPointRepository` | `domain::operation_point` |
| **Resampled Point (D1)** | An OHLCV candlestick bar resampled to the **daily (D1)** timeframe. Represents market price data for one day. | `ResampledPointD1` | `domain::resampled_point` |
| **Indicator** | A technical indicator applied to market data. All indicators in a backtest run must share the same `type`. | `Indicator` | `domain::indicator` |
| **Indicator Type** | The category/kind of a technical indicator (e.g. moving average, RSI). Represented as a string. | `String` field (`type`) | `domain::indicator` |
| **Money Management Strategy (MMS)** | A strategy that governs position sizing and risk. Identified by `money_management_strategy_id`. All operation points in a run must use the same MMS. | `i32` field | `domain::operation_point` |
| **Take Profit (TP)** | The target price level (in pips/points) at which a trade is closed for profit. | `i32` field (`tp`) | `domain::operation_point` |
| **Stop Loss (SL)** | The price level (in pips/points) at which a trade is closed to limit loss. | `i32` field (`sl`) | `domain::operation_point` |
| **Risk** | The fraction of capital risked on a single trade (e.g. `0.01` = 1%). | `f64` field | `domain::operation_point` |
| **Result** | The outcome of a trade: `1` = win, `-1` = loss. | `i32` field | `domain::operation_point` |
| **Timestamp** | An integer epoch identifying when a data point or trade occurred. Used to align operation points with resampled market data. | `i32` field | multiple |

---

## Core Entities

### OperationPoint

A single trade record. Used for both long and short positions (distinguished by which repository holds it).

```
OperationPoint {
    id:                          i32       — unique identifier
    instrument:                  String    — e.g. "EURUSD"
    result:                      i32       — trade outcome (1 = win, -1 = loss)
    tp:                          i32       — take profit level
    sl:                          i32       — stop loss level
    risk:                        f64       — fraction of capital risked
    money_management_strategy_id: i32      — which MMS governs this trade
    timestamp:                   i32       — when the trade occurred
}
```

### ResampledPointD1

Daily OHLCV candlestick bar — the market data backdrop against which trades are evaluated.

```
ResampledPointD1 {
    id:         i32       — unique identifier
    instrument: String    — e.g. "EURUSD"
    open:       f64       — opening price
    high:       f64       — highest price
    low:        f64       — lowest price
    close:      f64       — closing price
    volume:     i32       — trade volume
    timestamp:  i32       — day identifier
}
```

### Indicator

A technical indicator used in the strategy being backtested.

```
Indicator {
    id:   i32       — unique identifier
    type: String    — indicator category (e.g. "a")
}
```

---

## Repository Traits (Ports)

The domain defines **four repository traits** — these are the ports in a ports-and-adapters architecture:

| Trait | Returns | Semantics |
|-------|---------|-----------|
| `LongOperationPointRepository` | `Vec<OperationPoint>` | All long (buy) trades |
| `ShortOperationPointRepository` | `Vec<OperationPoint>` | All short (sell) trades |
| `ResampledPointD1Repository` | `Vec<ResampledPointD1>` | All daily candles |
| `IndicatorRepository` | `Vec<Indicator>` | All indicators |

Each trait has a single method: `get_all() -> Vec<T>`.

Infrastructure provides `InMemory*Repo` implementations for testing.

---

## Validation Rules (Domain Invariants)

Before a backtest can run, the input data must satisfy **five invariants**, checked in order by `validate_all()`:

### 1. Non-Empty Data (`empty_validation`)

All four data sets must be non-empty:
- Long operation points ≠ ∅
- Short operation points ≠ ∅
- Indicators ≠ ∅
- Resampled points ≠ ∅

**Rationale**: A backtest with missing data is meaningless.

### 2. Same Indicator Types (`same_indicator_types_validation`)

All indicators must share the same `type` value.

**Rationale**: A single backtest run evaluates one indicator type at a time.

### 3. Same Instrument (`same_instrument_validation`)

Every resampled point, long operation point, and short operation point must reference the **same instrument**. The first resampled point's instrument is the reference.

**Rationale**: You can't mix EURUSD trades with USDCAD market data.

### 4. Same Money Management Strategy (`same_mms_validation`)

Every operation point (long and short) must have a `money_management_strategy_id` matching the **expected MMS ID** passed as a parameter.

**Rationale**: A backtest run evaluates one MMS at a time.

### 5. Timestamp Alignment (`same_timestamp_validation`)

- Long and short operation points must have **identical timestamp sets**.
- Both must be a **subset** of the resampled points' timestamps.

**Rationale**: Every trade must correspond to a market data point, and longs and shorts must be evaluated over the same time window.

### Combined Validation

`combined_validation::validate_all()` runs all five checks in order, returning early on the first failure as a `CombinedValidationError`:

```
CombinedValidationError {
    Empty(EmptyRepoValidationError)   — which dataset is empty
    IndicatorTypeMismatch             — indicators have mixed types
    InstrumentMismatch                — data references different instruments
    MoneyManagementStrategyMismatch   — operation points use wrong MMS
    TimestampMismatch                 — timestamps don't align
}
```

---

## Architecture Layers

```
src/
├── cli/                       # CLI command tree (clap, one file per group)
│   ├── mod.rs                 # Coordinator: AppCommand enum, build_cli(), parse()
│   ├── dev.rs                 # dev create_indicators, create_strategies
│   ├── indicator.rs           # i/indicator create (macd/rsi), delete, delete_unused
│   ├── money_management_strategy.rs  # mms/money-management-strategy create, delete
│   ├── operation_point.rs     # o/operation-point align, create, delete
│   ├── raw_point.rs           # r/raw-point create, delete
│   ├── redis.rs               # redis get, delete
│   ├── resampled_point.rs     # rr/resampled-point create, delete
│   └── strategy.rs            # s/strategy create, create_one, delete, display_table, terminate, filters
├── engine/                    # Standalone Rust CLI engine (own binary)
│   ├── mod.rs                 # EngineCommand enum, build_cli(), parse(), execute()
│   └── strategy.rs            # strategy create subcommand
├── domain/                    # Pure domain logic — no I/O, no frameworks
│   ├── indicator.rs           # Indicator entity + repository trait
│   ├── operation_point.rs     # OperationPoint entity + long/short repo traits
│   ├── resampled_point.rs     # ResampledPointD1 entity + repo trait
│   ├── validation/            # Domain invariant checks
│   │   ├── empty_validation.rs
│   │   ├── same_indicator_types_validation.rs
│   │   ├── same_instrument_validation.rs
│   │   ├── same_mms_validation.rs
│   │   ├── same_timestamp_validation.rs
│   │   └── combined_validation.rs
│   └── fixtures.rs            # Test data builders
├── infrastructure/            # Adapters (in-memory repos)
│   └── in_memory_repos.rs
├── config/                    # Environment + DB configuration
│   └── environment.rs
├── bin/
│   └── bt-engine.rs           # bt-engine binary entry: engine::build_cli() → parse() → execute()
├── main.rs                    # bt binary entry: cli::build_cli() → parse() → execute()
└── lib.rs                     # Crate root — re-exports all modules (cli, config, domain, engine, infrastructure)
```

---

## Key Design Decisions Visible in Code

1. **Long/Short separation**: Longs and shorts use the same `OperationPoint` struct but are stored in separate repositories. This is a deliberate modeling choice — the direction (long vs short) is determined by *which repository* holds the point, not by a field on the struct.

2. **Validation as pure functions**: All validators are free functions taking slices, not methods on structs. This keeps them composable and testable without needing object construction.

3. **Fail-fast combined validation**: `validate_all` short-circuits on the first error. The order matters: emptiness is checked first (prevents index-out-of-bounds in later validators that access `[0]`).

4. **Fixtures module**: Test data lives in `domain::fixtures`, not in test files. This allows reuse across all validation test modules.

5. **Ports-and-adapters**: Repository traits live in the domain; implementations live in infrastructure. The domain has zero dependencies on infrastructure.

6. **Dual binary layout**: The project produces two binaries:
   - **`bt`** (`src/main.rs`) — the full backtesting CLI (dev, indicator, strategy, etc.). Uses `cli::build_cli()` -> `cli::parse()` -> `cli::execute()`.
   - **`bt-engine`** (`src/bin/bt-engine.rs`) — the standalone Rust engine CLI. Uses `engine::build_cli()` -> `engine::parse()` -> `engine::execute()`.
   
   Both entry points follow the same `build -> parse -> execute` pattern and return `anyhow::Result<()>`.

7. **Engine module separation**: The `engine/` module was extracted from `cli/mod.rs` to give the Rust engine its own command tree independent of the Python-wrapping `bt` CLI. Engine commands run directly in the `bt-engine` binary.

---

## How to Use This Document

- **When discussing trades**: Use "operation point", not "trade" or "signal". Specify "long" or "short".
- **When discussing market data**: Use "resampled point" or "D1 candle", not "price bar" or "OHLC".
- **When discussing risk rules**: Use "money management strategy" (MMS), not "position sizing" or "risk model".
- **When discussing validation**: Reference the specific invariant by name (e.g. "same-instrument validation").
- **When adding new entities**: Follow the pattern — struct + repository trait in domain, in-memory impl in infrastructure, fixtures in `domain::fixtures`.
