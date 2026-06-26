<!-- Context: project-intelligence/domain-model | Priority: critical | Version: 1.3 | Updated: 2026-06-26 -->

# Domain Model — Backtesting Engine

> Ubiquitous language, entity definitions, invariants, and validation rules extracted from the Rust codebase.
> This is the single source of truth for how we talk about the domain.

## Ubiquitous Language (Glossary)

| Term | Definition | Rust Type | Module |
|------|-----------|-----------|--------|
| **Instrument** | A tradeable financial pair (e.g. `EURUSD`, `USDCAD`). Identifies what is being traded. | `String` field | multiple |
| **Operation Point** | A single trade entry — a point in time where a position was opened. Contains the trade's result, take-profit, stop-loss, risk, and the money management strategy used. | `OperationPoint` | `domain::operation_point` |
| **Operation Points (bundle)** | A paired collection of long and short operation points. Provides timestamp-keyed lookup maps for aligning trades with market data. | `OperationPoints` | `domain::operation_point` |
| **Long Operation Point** | An operation point representing a **buy** (long) position. Stored in a separate repository from shorts. | `OperationPoint` via `LongOperationPointRepository` | `domain::operation_point` |
| **Short Operation Point** | An operation point representing a **sell** (short) position. Stored in a separate repository from longs. | `OperationPoint` via `ShortOperationPointRepository` | `domain::operation_point` |
| **Resampled Point (D1)** | An OHLCV candlestick bar resampled to the **daily (D1)** timeframe. Carries a `high_low_order` describing intraday price ordering. | `ResampledPointD1` | `domain::resampled_point` |
| **High-Low Order** | The intraday price ordering of a candle: `HighFirst`, `LowFirst`, or `Undefined`. | `HighLowOrder` | `domain::resampled_point` |
| **Indicator** | A technical indicator applied to market data. Has a `type`, typed `parameters` (Macd/Rsi), and a string `identifier`. All indicators in a backtest run must share the same `type`. | `Indicator` | `domain::indicator` |
| **Indicator Parameters** | Typed parameters for an indicator — either `Macd` (slow/fast type+n) or `Rsi` (n + high/low thresholds). | `IndicatorParameters` | `domain::indicator` |
| **Indicator Type** | The category/kind of a technical indicator (e.g. `macd`, `rsi`). | `String` field (`type`) | `domain::indicator` |
| **Money Management Strategy (MMS)** | A strategy that governs position sizing and risk. Identified by `money_management_strategy_id`. All operation points in a run must use the same MMS. | `i32` field | `domain::operation_point`, `domain::strategy` |
| **Take Profit (TP)** | The target price level (in pips/points) at which a trade is closed for profit. | `i32` field (`tp`) | `domain::operation_point` |
| **Stop Loss (SL)** | The price level (in pips/points) at which a trade is closed to limit loss. | `i32` field (`sl`) | `domain::operation_point` |
| **Risk** | The fraction of capital risked on a single trade (e.g. `0.01` = 1%). | `f64` field | `domain::operation_point` |
| **Result** | The outcome of a trade: `1` = win, `-1` = loss. | `i32` field | `domain::operation_point` |
| **Timestamp** | An integer epoch identifying when a data point or trade occurred. Used to align operation points with resampled market data. | `i32` field | multiple |
| **Strategy** | A backtested configuration: one indicator + a set of long/short operation point IDs + a money management strategy, annotated with performance metrics. | `Strategy` | `domain::strategy` |
| **Annual ROI** | Annualized return on investment for a strategy. Stored on `Strategy`; randomized in the current generator. | `f64` field | `domain::strategy` |
| **Max Draw Down** | Largest peak-to-trough decline of the cumulative-return curve, as a percentage. Computed from operation points; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::max_draw_down` |
| **Annual Operation Count** | Number of trades per year, annualized from the operation-point count over the time span. Computed from operation points; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::annual_operation_count` |
| **Global Strategy Filter** | Default thresholds for keeping a strategy: max draw down ≤ 30%, min annual ROI ≥ 10%, min annual operations ≥ 10. | `GlobalStrategyFilter` | `domain::strategy` |
| **Signal Group** | A collection of long and short signal timestamps for a single indicator. Maps an indicator ID to its signal timestamps. | `SignalGroup` | `infrastructure::redis` |
| **Signal Groups Cache** | An ephemeral Redis cache storing the complete mapping of indicator IDs to their signal groups. Prevents redundant computation across CLI invocations. | `SignalGroups` (HashMap) | `infrastructure::redis` |
| **Cache HIT / MISS** | Whether the Redis `signal_groups` key exists (HIT — skip compute) or not (MISS — compute from scratch). | CLI output | `engine::redis` |
| **Parquet Strategy Repository** | Writes `Vec<Strategy>` to partitioned parquet files under `{base}/instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. | `ParquetStrategyRepository` | `infrastructure::strategy_adapter` |
| **Strategy Generator** | Produces random `Strategy` records — either from DB-sourced indicators/operation points, or purely synthetic across (instrument, mms) partitions. | `generate_random_strategies`, `generate_random_strategies_bulk` | `application::strategy_generator` |
| **Strategy Data** | The fetched-from-PostgreSQL inputs for strategy generation: indicators, resampled points, long/short operation points for one (instrument, mms_id). | `StrategyData`, `StrategyQuery` | `application::strategy_data` |

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

### OperationPoints
A bundle pairing long and short operation points, with timestamp-keyed lookup maps.
```
OperationPoints {
    long_operation_points:  Vec<OperationPoint>
    short_operation_points: Vec<OperationPoint>
}
// Methods: long_points_by_timestamp()  -> FxHashMap<i32, &OperationPoint>
//          short_points_by_timestamp() -> FxHashMap<i32, &OperationPoint>
// Duplicate timestamps: last wins.
```

### ResampledPointD1
Daily OHLCV candlestick bar — the market data backdrop against which trades are evaluated.
```
ResampledPointD1 {
    id:              i32       — unique identifier
    instrument:      String    — e.g. "EURUSD"
    open:            f64       — opening price
    high:            f64       — highest price
    low:             f64       — lowest price
    close:           f64       — closing price
    volume:          i32       — trade volume
    timestamp:       i32       — day identifier
    high_low_order:  HighLowOrder — HighFirst | LowFirst | Undefined
}
```

### Indicator
A technical indicator used in the strategy being backtested.
```
Indicator {
    id:          i32                 — unique identifier
    r#type:      String              — indicator category ("macd" | "rsi")
    parameters:  IndicatorParameters — Macd{...} | Rsi{...}
    identifier:  String              — human-readable identifier
}
IndicatorParameters ::= Macd { slow_type, slow_n, fast_type, fast_n }
                      | Rsi { n, high_threshold, low_threshold }
```

### Strategy
A backtested configuration annotated with performance metrics.
```
Strategy {
    strategy_id:                   i64       — unique identifier
    indicator_id:                   i32       — which indicator this strategy uses
    annual_roi:                     f64       — annualized return on investment
    max_draw_down:                  f64       — largest peak-to-trough decline (%)
    annual_operation_count:         f64       — trades per year (annualized)
    long_operation_point_ids:       Vec<i32>  — long trades in this strategy
    short_operation_point_ids:      Vec<i32>  — short trades in this strategy
    money_management_strategy_id:   i32       — which MMS governs this strategy
}
```

### GlobalStrategyFilter
Default thresholds for `filter_strategies()`. `Default`: max_draw_down_max=0.30, min_annual_roi=0.10, min_operation_count=10.0.
```
GlobalStrategyFilter { max_draw_down_max: f64, min_annual_roi: f64, min_operation_count: f64 }
```

### SignalGroup
A group of signal timestamps (long and short) for a single indicator. Stored in Redis as JSON.
```
SignalGroup { long_signals: Vec<i64>, short_signals: Vec<i64> }
```

---

## Domain Metric Functions (Pure)

Pure functions in `domain/` — no I/O, no frameworks. Each is a free function over slices.

| Function | File | Signature | Behavior |
|----------|------|-----------|----------|
| `annual_operation_count` | `domain::annual_operation_count` | `(ops: &[OperationPoint], start: i32, end: i32) -> f64` | `(count / days) * 365.25`, rounded to 2dp. Returns 0.0 for empty ops or non-positive span. |
| `max_draw_down` | `domain::max_draw_down` | `(ops: &[OperationPoint]) -> f64` | Tracks cumulative return `1.0 + Σ(result*risk/sl)`, maintains running max, computes deepest `(cummax - cumsum)/cummax`, rounded to 2dp. Returns 0.0 for empty ops. |
| `filter_strategies` | `domain::strategy` | `(strategies: Vec<Strategy>, filter: &GlobalStrategyFilter) -> Vec<Strategy>` | Keeps strategies with `max_draw_down ≤ max`, `annual_roi ≥ min`, `annual_operation_count ≥ min`. |

---

## Repository Traits (Ports)

The domain defines repository traits — ports in a ports-and-adapters architecture:

| Trait | Methods | Semantics |
|-------|---------|-----------|
| `LongOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)` | Long (buy) trades |
| `ShortOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)` | Short (sell) trades |
| `ResampledPointD1Repository` | `get_all()`, `get_by_instrument(instrument)` | Daily candles |
| `IndicatorRepository` | `get_all()`, `get_by_type(indicator_type)` | Indicators |

Infrastructure provides `InMemory*Repo` (testing) and `Pg*Adapter` (PostgreSQL) implementations.

---

## Application Layer

`src/application/` — orchestration between domain and infrastructure (new since v1.2).

| Unit | File | Purpose |
|------|------|---------|
| `StrategyQuery` / `StrategyData` | `application::strategy_data` | `fetch_strategy_data(query)` connects to PG via `DbConfig::from_env`, fetches indicators/resampled/long/short for one (instrument, mms_id). Rich connection diagnostics on failure. |
| `generate_random_strategies` | `application::strategy_generator` | Builds one `Strategy` per DB-sourced indicator, picking random subsets (70–100%) of long/short op IDs. |
| `generate_random_strategies_bulk` | `application::strategy_generator` | Purely synthetic: distributes `count` strategies across `(instrument × mms_id)` partitions. Optional `seed` for reproducibility. No DB needed. |

---

## Infrastructure Adapters

| Adapter | Purpose | File |
|---------|---------|------|
| `Pg*Adapter` | PostgreSQL persistence (production) | `src/infrastructure/pg_adapter.rs` |
| `InMemory*Repo` | In-memory storage (testing) | `src/infrastructure/in_memory_repos.rs` |
| `Redis` module | Ephemeral cache for signal groups | `src/infrastructure/redis.rs` |
| `ParquetStrategyRepository` | Writes strategies to partitioned parquet files (ZSTD compression) | `src/infrastructure/strategy_adapter.rs` |

### Parquet Strategy Repository
Partition layout: `{base_dir}/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet`
- Schema: `strategy_id` (Int64), `indicator_id` (Int32), `annual_roi`/`max_draw_down`/`annual_operation_count` (Decimal128(10,2)), `long_operation_point_ids`/`short_operation_point_ids` (List<Int32>), `money_management_strategy_id` (Int32).
- `base_dir` resolved by `config::environment::parquet_base_dir(env)` → `/app/parquet_files/{env}/strategy`. Overridable via `--parquet-dir` for running outside Docker.

### Redis Cache
Three operations on a single key (`signal_groups`): `read_signal_groups`, `write_signal_groups`, `delete_signal_groups`. All hard-fail on connection/serialization error (no silent fallback).

---

## Validation Rules (Domain Invariants)

Before a backtest can run, the input data must satisfy **five invariants**, checked in order by `validate_all()`:

1. **Non-Empty Data** (`empty_validation`) — all four datasets non-empty.
2. **Same Indicator Types** (`same_indicator_types_validation`) — all indicators share the same `type`.
3. **Same Instrument** (`same_instrument_validation`) — every resampled/long/short point references the same instrument (first resampled point is the reference).
4. **Same Money Management Strategy** (`same_mms_validation`) — every operation point's `money_management_strategy_id` matches the expected MMS ID parameter.
5. **Timestamp Alignment** (`same_timestamp_validation`) — long and short timestamp sets are identical; both are subsets of resampled timestamps.

`combined_validation::validate_all()` runs all five in order, returning early on the first failure as a `CombinedValidationError` (Empty / IndicatorTypeMismatch / InstrumentMismatch / MoneyManagementStrategyMismatch / TimestampMismatch).

---

## Architecture Layers

```
src/
├── application/                # Orchestration between domain & infrastructure (new)
│   ├── mod.rs
│   ├── strategy_data.rs        # fetch_strategy_data() — PG-backed data assembly
│   └── strategy_generator.rs   # generate_random_strategies[_bulk]()
├── cli/                        # CLI command tree (clap, one file per group)
│   ├── mod.rs                  # Coordinator: AppCommand enum, build_cli(), parse(), execute(), exec_python_cli(), exec_engine_cli()
│   ├── dev.rs                  # dev create_indicators, create_strategies
│   ├── indicator.rs            # i/indicator create (macd/rsi), delete, delete_unused
│   ├── money_management_strategy.rs  # mms/money-management-strategy create, delete
│   ├── operation_point.rs     # o/operation-point align, create, delete
│   ├── raw_point.rs            # r/raw-point create, delete
│   ├── redis.rs                # ct redis — delegates to bt-engine redis via Docker exec
│   ├── resampled_point.rs      # rr/resampled-point create, delete
│   └── strategy.rs             # s/strategy create, generate, display (+verbose)
├── engine/                     # Standalone Rust CLI engine (own binary)
│   ├── mod.rs                  # EngineCommand enum, build_cli(), parse(), execute()
│   ├── redis.rs                # bt-engine redis — check HIT/MISS, verbose dump, --clean
│   └── strategy.rs             # bt-engine strategy create, generate
├── domain/                     # Pure domain logic — no I/O, no frameworks
│   ├── indicator.rs            # Indicator + IndicatorParameters + IndicatorRepository
│   ├── operation_point.rs      # OperationPoint + OperationPoints + long/short repo traits
│   ├── resampled_point.rs      # ResampledPointD1 + HighLowOrder + repo trait
│   ├── strategy.rs             # Strategy + GlobalStrategyFilter + filter_strategies()
│   ├── annual_operation_count.rs  # annual_operation_count() metric
│   ├── max_draw_down.rs        # max_draw_down() metric
│   ├── validation/             # Domain invariant checks (5 rules + combined)
│   └── fixtures.rs             # Test data builders
├── infrastructure/             # Adapters (PG, memory, Redis, parquet)
│   ├── in_memory_repos.rs
│   ├── pg_adapter.rs           # PG adapters implementing domain repo traits
│   ├── redis.rs                # SignalGroup types, SIGNAL_GROUPS_KEY, read/write/delete
│   └── strategy_adapter.rs     # ParquetStrategyRepository (partitioned parquet writes)
├── config/                     # Environment + DB configuration
│   └── environment.rs         # Environment, DbConfig, parquet_base_dir()
├── bin/
│   └── bt-engine.rs            # bt-engine binary entry: engine::build_cli() → parse() → execute()
├── main.rs                     # ct binary entry: cli::build_cli() → parse() → execute()
└── lib.rs                      # Crate root — re-exports application, cli, config, domain, engine, infrastructure
```

---

## Key Design Decisions Visible in Code

1. **Long/Short separation**: Longs and shorts use the same `OperationPoint` struct but are stored in separate repositories. Direction is determined by *which repository* holds the point, not by a field on the struct.
2. **Validation as pure functions**: All validators are free functions over slices, not methods on structs. Composable and testable without object construction.
3. **Fail-fast combined validation**: `validate_all` short-circuits on the first error. Order matters: emptiness is checked first (prevents index-out-of-bounds in later validators that access `[0]`).
4. **Fixtures module**: Test data lives in `domain::fixtures`, reusable across all test modules.
5. **Ports-and-adapters**: Repository traits live in the domain; implementations live in infrastructure. The domain has zero dependencies on infrastructure.
6. **Dual binary layout**: `ct` (`src/main.rs`) wraps the Python CLI; `bt-engine` (`src/bin/bt-engine.rs`) is the native Rust engine. Both follow `build -> parse -> execute` and return `anyhow::Result<()>`.
7. **Application layer introduced**: `src/application/` orchestrates domain + infrastructure (e.g. fetch-then-generate strategies). Keeps `engine/` CLI handlers thin.
8. **Metrics as pure domain functions**: `annual_operation_count` and `max_draw_down` are free functions over `&[OperationPoint]` in `domain/`. No I/O, no structs — composable and unit-tested in isolation.
9. **Strategy filtering as a pure function**: `filter_strategies(strategies, &GlobalStrategyFilter)` is a free function with a `Default`-able filter. Defaults encode the project's baseline quality bar.
10. **Parquet partitioning by (instrument, mms_id)**: Strategies are written as partitioned parquet files — `instrument=.../money_management_strategy_id=.../strategies.parquet`. Matches the single-instrument, single-MMS backtest constraint and enables efficient per-partition reads.
11. **Typed indicator parameters**: `IndicatorParameters` is an enum (`Macd`/`Rsi`) — the PG adapter hand-parses the JSON `parameters` column into the correct variant by `type`. No `serde` on the hot path.
12. **Simple Redis key constant**: `SIGNAL_GROUPS_KEY` is a plain `&str` (`"signal_groups"`). Hard fail on Redis errors — no silent fallback.
13. **Reproducible generation**: `generate_random_strategies_bulk` accepts an optional `seed` (StdRng) for deterministic test fixtures.

---

## How to Use This Document

- **When discussing trades**: Use "operation point", not "trade" or "signal". Specify "long" or "short".
- **When discussing market data**: Use "resampled point" or "D1 candle", not "price bar" or "OHLC".
- **When discussing risk rules**: Use "money management strategy" (MMS), not "position sizing" or "risk model".
- **When discussing validation**: Reference the specific invariant by name (e.g. "same-instrument validation").
- **When discussing cache**: Use "signal groups cache", "cache HIT/MISS". The cache is ephemeral — not a persistent store.
- **When discussing strategies**: Use "strategy" (a configured indicator + op-point sets + MMS + metrics). Reference "annual ROI", "max draw down", "annual operation count" by name.
- **When adding new entities**: Follow the pattern — struct + repository trait in domain, in-memory impl in infrastructure, fixtures in `domain::fixtures`.
- **When adding new metrics**: Follow `annual_operation_count`/`max_draw_down` — free function over `&[OperationPoint]` in `domain/`, unit-tested in the same file.
