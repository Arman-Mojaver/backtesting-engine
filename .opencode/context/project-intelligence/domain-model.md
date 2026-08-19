<!-- Context: project-intelligence/domain-model | Priority: critical | Version: 1.5 | Updated: 2026-08-19 -->

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
| **Timestamp** | An integer epoch (seconds) identifying when a data point or trade occurred. Used to align operation points with resampled market data. | `i64` field | multiple |
| **Strategy** | A backtested configuration: one indicator + a set of long/short operation point IDs + a money management strategy, annotated with performance metrics. | `Strategy` | `domain::strategy` |
| **Annual ROI** | Annualized return on investment for a strategy. Computed by compounding the global ROI over the time span; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::annual_roi` |
| **Max Draw Down** | Largest peak-to-trough decline of the cumulative-return curve, as a percentage. Computed from operation points via compounding against a running cummax; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::max_draw_down` |
| **Annual Operation Count** | Number of trades per year, annualized from a total count over the time span. Computed from signal-group operation counts; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::annual_operation_count` |
| **Global Strategy Filter** | Default thresholds for keeping a strategy: max draw down ≤ 30%, min annual ROI ≥ 10%. (Annual operation count is enforced earlier, at the signal-group stage.) | `GlobalStrategyFilter` | `domain::strategy` |
| **Signal Group** | A collection of long and short signal timestamps for a single indicator. Embeds the `indicator_id` it belongs to. Lives in the domain layer. | `SignalGroup` | `domain::signal_group` |
| **Parquet Strategy Repository** | Writes `Vec<Strategy>` to partitioned parquet files under `{base}/instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. | `ParquetStrategyRepository` | `infrastructure::strategy_adapter` |
| **Strategy Generator** | Builds `Strategy` records from signal groups + operation points, computing real metrics (`annual_roi`, `max_draw_down`, `annual_operation_count`) for each. Parallelized with rayon. | `create_strategies`, `create_strategy` | `application::strategy_generator` |
| **Strategy Data** | The fetched-from-PostgreSQL inputs for strategy generation: indicators, resampled points, long/short operation points for one (instrument, mms_id). | `StrategyData`, `StrategyQuery` | `application::strategy_data` |
| **Global ROI** | Total return on investment across a sequence of operation points, as a percentage. Compounds each trade's contribution against a running cummax. | `f64` (free fn) | `domain::global_roi` |
| **Signal Generator** | Produces `SignalGroup`s from indicators + resampled points by sampling random timestamps (excluding the first/last 200 candles). | `create_signals` | `application::signal_generator` |

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
    timestamp:                   i64       — when the trade occurred (epoch seconds)
}
```

### OperationPoints
A bundle pairing long and short operation points, with timestamp-keyed lookup maps.
```
OperationPoints {
    long_operation_points:  Vec<OperationPoint>
    short_operation_points: Vec<OperationPoint>
}
// Methods: long_points_by_timestamp()  -> FxHashMap<i64, &OperationPoint>
//          short_points_by_timestamp() -> FxHashMap<i64, &OperationPoint>
//          start_end_timestamps()      -> (i64, i64)  — (min, max) of long op timestamps
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
    timestamp:       i64       — day identifier (epoch seconds)
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
    indicator_id:                   i32       — which indicator this strategy uses
    annual_roi:                     f64       — annualized return on investment (%)
    max_draw_down:                  f64       — largest peak-to-trough decline (%)
    annual_operation_count:         f64       — trades per year (annualized)
    long_operation_point_ids:       Vec<i32>  — long trades in this strategy
    short_operation_point_ids:      Vec<i32>  — short trades in this strategy
    money_management_strategy_id:   i32       — which MMS governs this strategy
}
// Note: strategy_id is NOT a domain field. IDs are assigned at persistence time
// by ParquetStrategyRepository (global max + 1 across all partitions).
```

### GlobalStrategyFilter
Default thresholds for `filter_strategies()`. `Default`: max_draw_down_max=0.30, min_annual_roi=0.10, min_operation_count=10.0.
```
GlobalStrategyFilter { max_draw_down_max: f64, min_annual_roi: f64, min_operation_count: f64 }
```

### SignalGroup
A group of signal timestamps (long and short) for a single indicator. Embeds its `indicator_id`. Lives in the domain layer.
```
SignalGroup {
    indicator_id:  i32       — which indicator this group belongs to
    long_signals:  Vec<i64>  — long signal timestamps
    short_signals: Vec<i64>  — short signal timestamps
}
// Methods: operation_count() -> usize  (long.len() + short.len())
// Free fn: filter_signal_groups_by_annual_ops(groups, min_ops, start_ts, end_ts) -> Vec<SignalGroup>
//          Keeps groups whose annualized operation count >= min_ops and have > 0 ops.

---

## Domain Metric Functions (Pure)

Pure functions in `domain/` — no I/O, no frameworks. Each is a free function over slices.

| Function | File | Signature | Behavior |
|----------|------|-----------|----------|
| `annual_operation_count` | `domain::annual_operation_count` | `(total_count: usize, start_ts: i64, end_ts: i64) -> f64` | `total_count / years`, where `years = (end-start)/86400/365.25` (rounded 2dp). Returns 0.0 for zero count or non-positive span. |
| `max_draw_down` | `domain::max_draw_down` | `(ops: &[&OperationPoint]) -> f64` | Tracks `cumsum = 1 + Σ(cummax * result * risk/sl)`, maintains running `cummax`, computes deepest `(cummax - cumsum)/cummax` as a %, rounded to 2dp. Returns 0.0 for empty ops. |
| `global_roi` | `domain::global_roi` | `(ops: &[&OperationPoint]) -> f64` | Total ROI as a %: `cumsum = 1 + Σ(cummax * result * risk/sl)`, returns `(cumsum - 1) * 100`, rounded to 2dp. Returns 0.0 for empty ops. |
| `annual_roi` | `domain::annual_roi` | `(start_ts: i64, end_ts: i64, global_roi: f64) -> f64` | Annualizes the global ROI via compounding: `((1 + global_roi/100)^(1/years) - 1) * 100`, rounded to 2dp. Returns 0.0 for non-positive span. |
| `filter_signal_groups_by_annual_ops` | `domain::signal_group` | `(groups: Vec<SignalGroup>, min_ops: f64, start_ts: i64, end_ts: i64) -> Vec<SignalGroup>` | Keeps groups with `operation_count() > 0` and `annual_operation_count(count, start, end) >= min_ops`. |
| `filter_strategies` | `domain::strategy` | `(strategies: Vec<Strategy>, filter: &GlobalStrategyFilter) -> Vec<Strategy>` | Keeps strategies with `max_draw_down ≤ max` and `annual_roi ≥ min`. (Annual operation count is filtered earlier at the signal-group stage.) |

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

`src/application/` — orchestration between domain and infrastructure.

| Unit | File | Purpose |
|------|------|---------|
| `StrategyQuery` / `StrategyData` | `application::strategy_data` | `fetch_strategy_data(query)` connects to PG via `DbConfig::from_env`, fetches indicators/resampled/long/short for one (instrument, mms_id). Rich connection diagnostics on failure. |
| `create_signals` | `application::signal_generator` | Generates `SignalGroup`s from indicators + resampled points by sampling random timestamps (skips first/last 200 candles). Intercalates long/short, no repeated timestamps. |
| `create_strategies` | `application::strategy_generator` | Builds one `Strategy` per signal group, gathering referenced operation points and computing real metrics. Parallelized with **rayon** `par_iter()`. |
| `create_strategy` | `application::strategy_generator` | Single-strategy builder: resolves long/short op refs by timestamp, sorts by timestamp, computes `max_draw_down`, `global_roi` → `annual_roi`, and `annual_operation_count`. |

---

## Infrastructure Adapters

| Adapter | Purpose | File |
|---------|---------|------|
| `Pg*Adapter` | PostgreSQL persistence (production) | `src/infrastructure/pg_adapter.rs` |
| `InMemory*Repo` | In-memory storage (testing) | `src/infrastructure/in_memory_repos.rs` |
| `ParquetStrategyRepository` | Writes strategies to partitioned parquet files (ZSTD compression) | `src/infrastructure/strategy_adapter.rs` |

### Parquet Strategy Repository
Partition layout: `{base_dir}/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet`
- Schema: `strategy_id` (Int64), `indicator_id` (Int32), `annual_roi`/`max_draw_down`/`annual_operation_count` (Decimal128(10,2)), `long_operation_point_ids`/`short_operation_point_ids` (List<Int32>), `money_management_strategy_id` (Int32).
- `base_dir` resolved by `config::environment::parquet_base_dir(env)` → `/app/parquet_files/{env}/strategy`. Overridable via `--parquet-dir` for running outside Docker.
- **Append semantics**: `write()` reads existing rows (if file exists), appends new rows, rewrites the file. `strategy_id` is assigned at write time as `find_max_strategy_id() + 1` (global max across ALL partitions, scanned recursively). IDs are contiguous within a batch and continue across partitions and appends.


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
├── application/                # Orchestration between domain & infrastructure
│   ├── mod.rs
│   ├── signal_generator.rs    # create_signals() — random SignalGroup generation
│   ├── strategy_data.rs       # fetch_strategy_data() — PG-backed data assembly
│   └── strategy_generator.rs  # create_strategies() / create_strategy() — real metrics
├── cli/                        # CLI command tree (clap, one file per group)
│   ├── mod.rs                  # Coordinator: AppCommand enum, build_cli(), parse(), execute(), exec_python_cli(), exec_engine_cli()
│   ├── dev.rs                  # dev create_indicators, create_strategies
│   ├── indicator.rs            # i/indicator create (macd/rsi), delete, delete_unused
│   ├── money_management_strategy.rs  # mms/money-management-strategy create, delete
│   ├── operation_point.rs     # o/operation-point align, create, delete
│   ├── raw_point.rs            # r/raw-point create, delete
│   ├── resampled_point.rs      # rr/resampled-point create, delete
│   └── strategy.rs             # s/strategy create, display (+verbose)  [generate removed]
├── engine/                     # Standalone Rust CLI engine (own binary)
│   ├── mod.rs                  # EngineCommand enum, build_cli(), parse(), execute()
│   ├── strategy/                # bt-engine strategy commands (module dir)
│   │   ├── mod.rs              # StrategyCommand enum, build/parse/execute
│   │   └── create.rs           # strategy create — full pipeline (fetch→validate→signals→filter→strategies→parquet)
├── domain/                     # Pure domain logic — no I/O, no frameworks
│   ├── indicator.rs            # Indicator + IndicatorParameters + IndicatorRepository
│   ├── operation_point.rs      # OperationPoint + OperationPoints + long/short repo traits
│   ├── resampled_point.rs      # ResampledPointD1 + HighLowOrder + repo trait
│   ├── signal_group.rs         # SignalGroup + filter_signal_groups_by_annual_ops()
│   ├── strategy.rs             # Strategy + GlobalStrategyFilter + filter_strategies()
│   ├── annual_operation_count.rs  # annual_operation_count() metric
│   ├── annual_roi.rs           # annual_roi() metric
│   ├── global_roi.rs           # global_roi() metric
│   ├── max_draw_down.rs        # max_draw_down() metric
│   ├── validation/             # Domain invariant checks (5 rules + combined)
│   └── fixtures.rs             # Test data builders
├── infrastructure/             # Adapters (PG, memory, parquet)
│   ├── in_memory_repos.rs
│   ├── pg_adapter.rs           # PG adapters implementing domain repo traits
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
8. **Metrics as pure domain functions**: `annual_operation_count`, `max_draw_down`, `global_roi`, and `annual_roi` are free functions in `domain/`. `max_draw_down` and `global_roi` take `&[&OperationPoint]`; `annual_operation_count` takes a count + timestamp span; `annual_roi` takes a span + global ROI. No I/O, no structs — composable and unit-tested in isolation.
9. **Two-stage filtering**: `filter_signal_groups_by_annual_ops()` prunes signal groups by annual operation count first (cheap), then `filter_strategies(strategies, &GlobalStrategyFilter)` prunes by max draw down + annual ROI (expensive, runs on fewer candidates). `GlobalStrategyFilter::default()` encodes the baseline quality bar.
10. **Parquet partitioning by (instrument, mms_id)**: Strategies are written as partitioned parquet files — `instrument=.../money_management_strategy_id=.../strategies.parquet`. Matches the single-instrument, single-MMS backtest constraint and enables efficient per-partition reads.
11. **Typed indicator parameters**: `IndicatorParameters` is an enum (`Macd`/`Rsi`) — the PG adapter hand-parses the JSON `parameters` column into the correct variant by `type`. No `serde` on the hot path.
12. **Real metrics in the generator**: `create_strategies()` computes `max_draw_down`, `global_roi` → `annual_roi`, and `annual_operation_count` from each strategy's referenced operation points — no more random placeholder metrics. Parallelized with **rayon** `par_iter()`.
13. **`SignalGroup` lives in the domain**: `SignalGroup` was moved from infrastructure to `domain::signal_group` and now embeds `indicator_id`. The cache stores `Vec<SignalGroup>` (not a HashMap keyed by indicator ID) — the ID travels with each group.
14. **Timestamps are `i64`**: All timestamps (`OperationPoint`, `ResampledPointD1`, signal timestamps) are `i64` epoch seconds, not `i32`. The PG adapter casts `i32` DB columns to `i64` on read.
15. **`strategy_id` is persistence-assigned, not domain-owned**: The `Strategy` struct has no `strategy_id` field. IDs are assigned at parquet write time by `ParquetStrategyRepository` (global max across all partitions + 1). Keeps the domain free of persistence concerns.
16. **Parquet append semantics**: `write()` reads existing rows, appends new ones, and rewrites the whole file. `find_max_strategy_id()` recursively scans all partitions to find the global max ID, ensuring IDs are unique and contiguous across partitions and appends.
17. **`strategy generate` command removed**: The purely-synthetic `generate` command (both `ct s generate` and `bt-engine strategy generate`) has been removed. Strategy creation now flows through `strategy create`, which fetches real data from PG, generates signals, and computes real metrics.

---

## How to Use This Document

- **When discussing trades**: Use "operation point", not "trade" or "signal". Specify "long" or "short".
- **When discussing market data**: Use "resampled point" or "D1 candle", not "price bar" or "OHLC".
- **When discussing risk rules**: Use "money management strategy" (MMS), not "position sizing" or "risk model".
- **When discussing validation**: Reference the specific invariant by name (e.g. "same-instrument validation").
- **When discussing cache**: Use "signal-group memoization". The `cached` crate memoizes per `(indicator_id, instrument)` in-process.
- **When discussing strategies**: Use "strategy" (a configured indicator + op-point sets + MMS + metrics). Reference "annual ROI", "max draw down", "annual operation count" by name.
- **When adding new entities**: Follow the pattern — struct + repository trait in domain, in-memory impl in infrastructure, fixtures in `domain::fixtures`.
- **When adding new metrics**: Follow `annual_operation_count`/`max_draw_down`/`global_roi`/`annual_roi` — free function in `domain/`, unit-tested in the same file. `max_draw_down` and `global_roi` take `&[&OperationPoint]`; `annual_operation_count` takes a count + timestamp span; `annual_roi` takes a span + global ROI.
