<!-- Context: project-intelligence/domain-model | Priority: critical | Version: 1.11 | Updated: 2026-10-23 -->

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
| **Indicator** | A technical indicator applied to market data. Has an `id` and typed `parameters`. | `Indicator` | `domain::indicator` |
| **Indicator Parameters** | Typed parameters for an indicator — a 14-variant enum: `Macd`, `Rsi`, `AwesomeOscillator`, `AcceleratorDecelerator`, `BearsPower`, `BullsPower`, `ChaikinOscillator`, `ChaikinVolatility`, `DetrendedPriceOscillator`, `ForceIndex`, `OsMa`, `Roc`, `Trix`, `Vroc`. | `IndicatorParameters` | `domain::indicator` |
| **Indicator Type** | The category/kind of a technical indicator (e.g. `macd`, `rsi`). Not a domain-struct field: lives in the database `type` column, which the PG adapter reads to select the `IndicatorParameters` variant. | `type` DB column | `infrastructure::pg_adapter` |
| **Indicator Output** | The computed result of an indicator over a candle series — `Oscillator`, `Threshold`, or `Crossover` — carrying timestamped values. Converted to a `SignalGroup` by signal semantics. | `IndicatorOutput` | `domain::signal` |
| **Money Management Strategy (MMS)** | A strategy that governs position sizing and risk. Identified by `money_management_strategy_id`. All operation points in a run must use the same MMS. | `i32` field | `domain::operation_point`, `domain::strategy` |
| **Take Profit (TP)** | The target price level (in pips/points) at which a trade is closed for profit. | `i32` field (`tp`) | `domain::operation_point` |
| **Stop Loss (SL)** | The price level (in pips/points) at which a trade is closed to limit loss. | `i32` field (`sl`) | `domain::operation_point` |
| **Risk** | The fraction of capital risked on a single trade (e.g. `0.01` = 1%). | `f64` field | `domain::operation_point` |
| **Result** | The outcome of a trade: `1` = win, `-1` = loss. | `i32` field | `domain::operation_point` |
| **Timestamp** | An integer epoch (seconds) identifying when a data point or trade occurred. Used to align operation points with resampled market data. | `i64` field | multiple |
| **Strategy** | A backtested configuration: one indicator + a set of long/short operation point IDs + a money management strategy, annotated with performance metrics. Carries its `instrument`. | `Strategy` | `domain::strategy` |
| **Annual ROI** | Annualized return on investment for a strategy. Computed by compounding the global ROI over the time span; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::performance_metrics` (method) |
| **Max Draw Down** | Largest peak-to-trough decline of the cumulative-return curve, as a percentage. Computed from operation points via compounding against a running cummax; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::performance_metrics` |
| **Annual Operation Count** | Number of trades per year, annualized from a total count over the time span. Computed from signal-group operation counts; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::annual_operation_count` |
| **Global Strategy Filter** | Default thresholds for keeping a strategy: max draw down ≤ 30%, min annual ROI ≥ 5%. (Annual operation count ≥ 10 is enforced earlier, at the signal-group stage.) | `GlobalStrategyFilter` | `domain::strategy` |
| **Signal Group** | A collection of long and short signal timestamps for a single indicator. Embeds the `indicator_id` and `instrument` it belongs to, plus a precomputed `operation_count`. Lives in the domain layer. | `SignalGroup` | `domain::signal_group` |
| **Cell** | One `(instrument, mms_id)` pair — the unit of work for `create_loop`. Cells are computed in parallel and persisted as one Parquet partition each. | `(String, i32)` keys, `CellOutput` | `application::cell_output`, `engine::strategy::create_loop` |
| **Parquet Strategy Repository** | Writes strategies to partitioned parquet files under `{base}/instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. Dedups fresh rows by `indicator_id` (idempotent partitions). | `ParquetStrategyRepository` | `infrastructure::strategy_adapter` |
| **Processed Manifest** | Parquet ledger of completed `(instrument, mms_id, indicator_id)` triples with `processed_at` timestamps. Makes `create`/`create_loop` idempotent. Atomic writes (temp + rename); dedup per triple. | `ParquetProcessedManifest` | `infrastructure::processed_manifest` |
| **Strategy Persistence** | Batch commit for `create_loop`: assigns `strategy_id`s from the global max, writes partitions in parallel (rayon), commits the manifest once. | `StrategyPersistence` | `infrastructure::strategy_persistence` |
| **Strategy Runner** | The core use case: validate → signal groups → filter by annual ops → create strategies (real metrics) → filter by DD/ROI. | `run_strategies` | `application::strategy_runner` |
| **Strategy Generator** | Builds `Strategy` records from signal groups + operation points, computing real metrics (`annual_roi`, `max_draw_down`, `annual_operation_count`) for each. Parallelized with rayon. | `create_strategies`, `create_strategy` | `application::strategy_generator` |
| **Source Data** | The fetched-from-PostgreSQL inputs for strategy generation: indicators, resampled points, long/short operation points for one (instrument, mms_id). | `SourceData`, `fetch_source_data` | `application::source_data` |
| **Metadata Report** | DB-vs-manifest progress report: which (instrument, mms_id, indicator_id) triples are processed and which remain. | `build_metadata_report` | `application::metadata_report` |
| **Global ROI** | Total return on investment across a sequence of operation points, as a percentage. Compounds each trade's contribution against a running cummax. Computed together with max draw down in one pass. | `f64` field on `PerformanceMetrics` | `domain::performance_metrics` |
| **Signal Generator** | Produces `SignalGroup`s from indicators + resampled points by computing real indicator outputs (`create_indicator_output`) and applying signal semantics (zero-cross / threshold / crossover). Memoized per `(indicator_id, instrument)` via the `cached` crate. | `create_signal_groups` | `application::signal_generator` |

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
    long_operation_points:  &[OperationPoint]
    short_operation_points: &[OperationPoint]
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
    parameters:  IndicatorParameters — 14-variant typed enum (see below)
}
IndicatorParameters ::= Macd { slow_type, slow_n, fast_type, fast_n }
                      | Rsi { n, high_threshold, low_threshold }
                      | AwesomeOscillator
                      | AcceleratorDecelerator
                      | BearsPower { period }
                      | BullsPower { period }
                      | ChaikinOscillator { fast_ma, slow_ma }
                      | ChaikinVolatility { smooth_period, chv_period }
                      | DetrendedPriceOscillator { period }
                      | ForceIndex { period }
                      | OsMa { fast_ema, slow_ema, signal_sma }
                      | Roc { period }
                      | Trix { period }
                      | Vroc { period }
```

### IndicatorOutput (domain::signal)
The computed result of an indicator over a candle series, and its signal semantics:
```
IndicatorOutput ::= Oscillator { instrument, buffer: OscillatorSeries }
                  | Threshold  { instrument, buffer: ThresholdSeries }
                  | Crossover  { instrument, buffer: CrossoverPair }

IndicatorValue { timestamp: i64, value: f64 }
OscillatorSeries { items: Vec<IndicatorValue> }
ThresholdSeries  { items, high_threshold, low_threshold }
CrossoverPair    { upcross_long_buffer, upcross_short_buffer }

// IndicatorOutput::create_signal_group(indicator_id) -> SignalGroup
// Signal rules (domain/signal/):
//   Oscillator: prev < 0 && curr > 0 -> long; prev > 0 && curr < 0 -> short (zero-cross)
//   Threshold:  prev < low && curr > low -> long; prev > high && curr < high -> short
//   Crossover:  (long_buffer - short_buffer) diff, then zero-cross
```

### Strategy
A backtested configuration annotated with performance metrics.
```
Strategy {
    instrument:                     String    — which instrument this strategy ran on
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
Default thresholds for `filter_strategies()`. `Default`: max_draw_down_max=30.0, min_annual_roi=5.0, min_operation_count=10.0.
```
GlobalStrategyFilter { max_draw_down_max: f64, min_annual_roi: f64, min_operation_count: f64 }
```

### SignalGroup
A group of signal timestamps (long and short) for a single indicator. Embeds its `indicator_id` and `instrument`, plus a precomputed `operation_count`. Lives in the domain layer.
```
SignalGroup {
    indicator_id:     i32       — which indicator this group belongs to
    long_signals:     Vec<i64>  — long signal timestamps
    short_signals:    Vec<i64>  — short signal timestamps
    operation_count:  usize     — long.len() + short.len()
    instrument:       String    — which instrument the signals were computed on
}
// Free fn: filter_signal_groups_by_annual_ops(groups, min_ops, start_ts, end_ts) -> Vec<SignalGroup>
//          Keeps groups whose annualized operation count >= min_ops and have > 0 ops.
```

---

## Domain Metric Functions (Pure)

Pure functions in `domain/` — no I/O, no frameworks. `performance_metrics()` returns a `PerformanceMetrics` struct (both curve-derived metrics in one pass); the others are free functions over plain values.

| Function | File | Signature | Behavior |
|----------|------|-----------|----------|
| `annual_operation_count` | `domain::annual_operation_count` | `(total_count: usize, start_ts: i64, end_ts: i64) -> f64` | `total_count / years`, where `years = (end-start)/86400/365.25` (rounded 2dp). Returns 0.0 for zero count or non-positive span. |
| `performance_metrics` | `domain::performance_metrics` | `(ops: &[&OperationPoint]) -> PerformanceMetrics` | Sorts ops chronologically itself (stable; ties keep input order), then ONE pass: `cumsum = 1 + Σ(cummax * result * risk/sl)` with running `cummax`. Returns `{ global_roi: (cumsum - 1) * 100, max_draw_down: deepest (cummax - cumsum)/cummax * 100 }`, each rounded to 2dp independently. Both 0.0 for empty ops. |
| `PerformanceMetrics::annual_roi` | `domain::performance_metrics` | `(&self, start_ts: i64, end_ts: i64) -> f64` | Annualizes `self.global_roi` via compounding: `((1 + roi/100)^(1/years) - 1) * 100`, rounded to 2dp. Returns 0.0 for non-positive span. |
| `filter_signal_groups_by_annual_ops` | `domain::signal_group` | `(groups: Vec<SignalGroup>, min_ops: f64, start_ts: i64, end_ts: i64) -> Vec<SignalGroup>` | Keeps groups with `operation_count > 0` and `annual_operation_count(count, start, end) >= min_ops`. |
| `filter_strategies` | `domain::strategy` | `(strategies: Vec<Strategy>, filter: &GlobalStrategyFilter) -> Vec<Strategy>` | Keeps strategies with `max_draw_down ≤ max` and `annual_roi ≥ min`. (Annual operation count is filtered earlier at the signal-group stage.) |

---

## Indicator Computation (domain::indicators)

`create_indicator_output(parameters, points) -> Result<IndicatorOutput, IndicatorError>` dispatches on the typed parameter enum:

| Indicator | Output kind | Notes |
|-----------|-------------|-------|
| Rsi | Threshold | over close prices |
| AwesomeOscillator, AcceleratorDecelerator, BearsPower, BullsPower, ChaikinOscillator, ChaikinVolatility, DetrendedPriceOscillator, ForceIndex, OsMa, Roc, Trix, Vroc | Oscillator | one file each in `domain/indicators/` |
| Macd | — | **Unsupported**: returns `IndicatorError::UnsupportedIndicator`; the signal generator skips it (`filter_map`), the run continues |

- `IndicatorError` has typed per-indicator period variants + `NotEnoughData` + `UnsupportedIndicator`.
- Period validation is uniform via the `validate_period!` macro (non-positive period → typed error).
- Insufficient data yields an **empty series**, not an error (`empty_oscillator!` macro).
- Shared moving-average math lives in `domain/indicators/moving_averages.rs`.
- `PriceField` (Open/High/Low/Close) selects which candle component an indicator consumes.

---

## Repository Traits (Ports)

The domain defines repository traits — ports in a ports-and-adapters architecture:

| Trait | Methods | Semantics |
|-------|---------|-----------|
| `LongOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)`, `index_by_cell()` (default) | Long (buy) trades; `index_by_cell` groups by `(instrument, mms_id)` |
| `ShortOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)`, `index_by_cell()` (default) | Short (sell) trades |
| `ResampledPointD1Repository` | `get_all()`, `get_by_instrument(instrument)`, `index_by_instrument()` (default) | Daily candles |
| `IndicatorRepository` | `get_all()` | Indicators |

Infrastructure provides the `Pg*Adapter` (PostgreSQL) implementations. The in-memory test repos were removed (2026-08); tests use `domain::fixtures` builders + `tempfile`-based adapter tests instead. There is no dedicated money-management-strategy adapter: `strategy metadata` currently derives distinct money-management ids from the long operation-point table (`PgLongOperationPointAdapter::get_distinct_mms_ids()`); this will change in Ticket 6, which unions the long and short sources.

PG adapters expose extra read methods beyond the traits: `get_all_indicator_ids()`, `get_distinct_instruments()`, `get_distinct_mms_ids()` (used by `strategy metadata` and `create_loop`).

---

## Application Layer

`src/application/` — orchestration between domain and infrastructure. One file per use case.

| Unit | File | Purpose |
|------|------|---------|
| `fetch_source_data` | `application::source_data` | Connects to PG via `DbConfig::from_env`, fetches indicators/resampled/long/short for one (instrument, mms_id). Rich connection diagnostics on failure. |
| `create_signal_groups` | `application::signal_generator` | Computes REAL indicator outputs (`create_indicator_output`) and converts them to `SignalGroup`s via signal semantics. Rayon `par_iter`; per-indicator memoization via `#[cached]` keyed `(indicator_id, instrument)`. Unsupported indicators are skipped, not aborted. |
| `create_strategies` / `create_strategy` | `application::strategy_generator` | Builds one `Strategy` per signal group, gathering referenced operation points and computing real metrics. Parallelized with **rayon** `par_iter()`. |
| `run_strategies` | `application::strategy_runner` | The core use case: `validate_all` → `create_signal_groups` → `filter_signal_groups_by_annual_ops` → `create_strategies` → `filter_strategies`. |
| `get_unprocessed` | `application::strategy_runner_utils` | Set-diff of all indicator IDs vs manifest-processed IDs. |
| `CellOutput` | `application::cell_output` | One cell's compute result: instrument, mms_id, strategies, processed indicator IDs. |
| `build_metadata_report` | `application::metadata_report` | DB state vs manifest state → remaining (instrument, mms_id, indicator_id) triples report. |

---

## Infrastructure Adapters

| Adapter | Purpose | File |
|---------|---------|------|
| `Pg*Adapter` | PostgreSQL persistence (production) | `src/infrastructure/pg_adapter.rs` |
| `ParquetStrategyRepository` | Writes strategies to partitioned parquet files (ZSTD compression) | `src/infrastructure/strategy_adapter.rs` |
| `ParquetProcessedManifest` | Idempotency ledger of processed (instrument, mms_id, indicator_id) triples | `src/infrastructure/processed_manifest.rs` |
| `StrategyPersistence` | Batch commit for create_loop: ID assignment + parallel partition writes + manifest commit | `src/infrastructure/strategy_persistence.rs` |
| `logging` | simplelog initialization (term + file, RUST_LOG-driven) | `src/infrastructure/logging.rs` |

### Parquet Strategy Repository
Partition layout: `{base_dir}/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet`
- Schema: `strategy_id` (Int64), `indicator_id` (Int32), `annual_roi`/`max_draw_down`/`annual_operation_count` (Decimal128(10,2)), `long_operation_point_ids`/`short_operation_point_ids` (List<Int32>), `money_management_strategy_id` (Int32).
- `base_dir` resolved by `config::environment::parquet_base_dir(env)` → `/app/parquet_files/{env}/strategy`.
- **Idempotent partitions**: `write_partition` reads existing rows and drops fresh rows whose `indicator_id` already exists — re-runs never duplicate.
- **Append semantics**: write = read existing + append fresh + rewrite the file. `strategy_id` assigned at write time as `max_strategy_id() + 1` (global max across ALL partitions, scanned recursively). IDs are contiguous within a batch and continue across partitions and appends.
- API: `write(instrument, mms_id, &[Strategy])` (assigns IDs), `write_rows(instrument, mms_id, Vec<StrategyRow>)` (pre-assigned IDs, used by `StrategyPersistence`), `max_strategy_id()`.

### Processed Manifest
`{root_dir}/processed.parquet` — schema: `instrument` (Utf8), `mms_id` (Int32), `indicator_id` (Int32), `processed_at` (Int64).
- `read(instrument, mms_id)`, `read_processed_ids_per_cell()`, `append(...)`, `append_all(entries, processed_at)`.
- Dedups on the (instrument, mms_id, indicator_id) triple; atomic writes (temp file + rename).
- Consumed by `strategy create` (skip processed), `create_loop` (per-cell skip), `strategy metadata` (progress report).

---

## Validation Rules (Domain Invariants)

Before a backtest can run, the input data must satisfy **five invariants**, checked in order by `validate_all()`:

1. **Non-Empty Data** (`empty_validation`) — all four datasets non-empty.
2. **Same Indicator Types** (`same_indicator_types_validation`) — all indicators share the same `type`.
3. **Same Instrument** (`same_instrument_validation`) — every resampled/long/short point references the same instrument (itertools `all_equal()` over the chained iterators).
4. **Same Money Management Strategy** (`same_mms_validation`) — every operation point's `money_management_strategy_id` matches the expected MMS ID parameter.
5. **Timestamp Alignment** (`same_timestamp_validation`) — long and short timestamp sets are identical; both are subsets of resampled timestamps.

`combined_validation::validate_all()` runs all five in order, returning early on the first failure as a `CombinedValidationError` (Empty / IndicatorTypeMismatch / InstrumentMismatch / MoneyManagementStrategyMismatch / TimestampMismatch). Order is load-bearing: emptiness first — otherwise the `all_equal()`-based same-X checks would vacuously pass on empty input.

---

## Architecture Layers

```
src/
├── application/                # Orchestration between domain & infrastructure
│   ├── mod.rs
│   ├── cell_output.rs          # CellOutput — one (instrument, mms_id) cell's compute result
│   ├── metadata_report.rs      # DB vs manifest progress report
│   ├── signal_generator.rs     # create_signal_groups() — REAL indicator-derived signals
│   ├── source_data.rs          # fetch_source_data() — PG-backed data assembly
│   ├── strategy_generator.rs   # create_strategies() / create_strategy() — real metrics
│   ├── strategy_runner.rs      # run_strategies() — validate → signals → filter → strategies → filter
│   └── strategy_runner_utils.rs # get_unprocessed() — manifest diffing
├── domain/                     # Pure domain logic — no I/O, no frameworks
│   ├── indicator.rs            # Indicator + IndicatorParameters (14 variants) + IndicatorRepository
│   ├── indicators/             # Indicator computation: 14 indicators + dispatch + macros + moving averages
│   │   ├── mod.rs              # create_indicator_output() dispatch, PriceField, IndicatorError
│   │   ├── macros.rs           # indicator_values!, validate_period!, empty_oscillator!
│   │   ├── moving_averages.rs  # shared MA math
│   │   ├── rsi.rs, ao.rs, ac.rs, bears.rs, bulls.rs, cho.rs, chv.rs,
│   │   ├── dpo.rs, force.rs, osma.rs, roc.rs, trix.rs, vroc.rs
│   │   └── test_support.rs     # #[cfg(test)] shared test helpers
│   ├── operation_point.rs      # OperationPoint + OperationPoints + long/short repo traits (+ index_by_cell)
│   ├── resampled_point.rs      # ResampledPointD1 + HighLowOrder + repo trait (+ index_by_instrument)
│   ├── signal/                 # Signal semantics: IndicatorOutput → SignalGroup
│   │   ├── mod.rs              # IndicatorOutput enum + create_signal_group()
│   │   ├── oscillator.rs       # zero-cross signals
│   │   ├── thresholds.rs       # threshold-crossing signals
│   │   └── crossover.rs        # diff-of-series → zero-cross signals
│   ├── signal_group.rs         # SignalGroup + filter_signal_groups_by_annual_ops()
│   ├── strategy.rs             # Strategy + GlobalStrategyFilter + filter_strategies()
│   ├── annual_operation_count.rs  # annual_operation_count() metric
│   ├── performance_metrics.rs  # performance_metrics() -> PerformanceMetrics (single pass) + annual_roi() method
│   ├── validation/             # Domain invariant checks (5 rules + combined)
│   └── fixtures.rs             # Test data builders
├── engine/                     # Standalone Rust CLI engine (own binary)
│   ├── mod.rs                  # EngineCommand enum, build_cli(), parse(), execute()
│   └── strategy/               # bt-engine strategy commands
│       ├── mod.rs              # StrategyCommand enum (create, create_loop, metadata)
│       ├── create.rs           # strategy create — single cell pipeline
│       ├── create_loop.rs      # strategy create_loop — all cells, parallel, idempotent
│       └── metadata.rs         # strategy metadata — DB vs manifest report
├── infrastructure/             # Adapters (PG, parquet, manifest, logging)
│   ├── pg_adapter.rs           # PG adapters implementing domain repo traits
│   ├── strategy_adapter.rs     # ParquetStrategyRepository (partitioned, idempotent parquet writes)
│   ├── processed_manifest.rs   # ParquetProcessedManifest (idempotency ledger)
│   ├── strategy_persistence.rs # StrategyPersistence (batch commit)
│   └── logging.rs              # simplelog init
├── config/                     # Environment + DB configuration
│   └── environment.rs          # Environment, DbConfig, parquet_base_dir(), parquet_root_dir()
├── bin/
│   └── bt-engine.rs            # bt-engine binary entry: engine::build_cli() → parse() → execute()
└── lib.rs                      # Crate root — re-exports application, config, domain, engine, infrastructure
```

---

## Key Design Decisions Visible in Code

1. **Long/Short separation**: Longs and shorts use the same `OperationPoint` struct but are stored in separate repositories. Direction is determined by *which repository* holds the point, not by a field on the struct.
2. **Validation as pure functions**: All validators are free functions over slices, not methods on structs. Composable and testable without object construction.
3. **Fail-fast combined validation**: `validate_all` short-circuits on the first error. Order matters: emptiness is checked first — otherwise the `all_equal()`-based same-X checks would vacuously pass on empty input.
4. **Fixtures module**: Test data lives in `domain::fixtures`, reusable across all test modules. (In-memory repos were removed 2026-08 — fixtures + tempfile cover the same need with less code.)
5. **Ports-and-adapters**: Repository traits live in the domain; implementations live in infrastructure. The domain has zero dependencies on infrastructure.
6. **Single-binary engine layout**: `bt-engine` (`src/bin/bt-engine.rs`) is the sole Rust CLI; it follows the `build -> parse -> execute` pattern and returns `anyhow::Result<()>`. The previous `ct` operator frontend (which forwarded engine commands via Docker exec and Python CLI commands via the `api` container) was removed in the commit that dropped `src/cli/` and `src/main.rs` — data CRUD and the iteration table are now reached exclusively in the companion Python service.
7. **Application layer**: `src/application/` orchestrates domain + infrastructure. Keeps `engine/` CLI handlers thin. One file per use case.
8. **Metrics as pure domain functions**: `performance_metrics()` takes `&[&OperationPoint]` and returns `PerformanceMetrics` (`global_roi` + `max_draw_down`) computed in a single pass over the shared compounding curve, sorting its input chronologically itself (stable; ties keep input order). `annual_roi` is a method on `PerformanceMetrics`. No I/O — composable and unit-tested in isolation.
9. **Two-stage filtering**: `filter_signal_groups_by_annual_ops()` prunes signal groups by annual operation count first (cheap), then `filter_strategies(strategies, &GlobalStrategyFilter)` prunes by max draw down + annual ROI (expensive, runs on fewer candidates). `GlobalStrategyFilter::default()` encodes the baseline quality bar (DD ≤ 30%, ROI ≥ 5%, ops ≥ 10/yr).
10. **Parquet partitioning by (instrument, mms_id)**: Strategies are written as partitioned parquet files — `instrument=.../money_management_strategy_id=.../strategies.parquet`. Matches the single-instrument, single-MMS backtest constraint and enables efficient per-partition reads.
11. **Typed indicator parameters**: `IndicatorParameters` is a 14-variant enum — the PG adapter hand-parses the JSON `parameters` column into the correct variant by `type`. No `serde` on the hot path.
12. **Real metrics in the generator**: `create_strategies()` computes `performance_metrics()` → `annual_roi()`, plus `annual_operation_count`, from each strategy's referenced operation points. Parallelized with **rayon** `par_iter()`.
13. **`SignalGroup` lives in the domain**: `SignalGroup` embeds `indicator_id`, `instrument`, and a precomputed `operation_count`. The cache stores `Vec<SignalGroup>` — IDs travel with each group.
14. **Timestamps are `i64`**: All timestamps are `i64` epoch seconds. The PG adapter casts `i32` DB columns to `i64` on read.
15. **`strategy_id` is persistence-assigned, not domain-owned**: The `Strategy` struct has no `strategy_id` field. IDs are assigned at parquet write time (global max across all partitions + 1). Keeps the domain free of persistence concerns.
16. **Idempotent parquet partitions**: `write_partition` dedups fresh rows by `indicator_id` against existing rows — re-runs never duplicate strategies for the same indicator.
17. **Processed manifest for idempotency**: `ParquetProcessedManifest` records completed (instrument, mms_id, indicator_id) triples; `create`/`create_loop` only compute the unprocessed remainder. Atomic writes (temp + rename).
18. **Real signal generation**: `create_signal_groups()` computes actual indicator outputs (`create_indicator_output`) and derives signals via zero-cross / threshold / crossover semantics. MACD is explicitly unsupported and skipped, not aborted.
19. **Batch parallelism in `create_loop`**: All four source tables loaded once via nested `rayon::join`, indexed in memory; cells computed via `par_iter`; persistence assigns IDs once, writes partitions in parallel, commits the manifest once.

---

## How to Use This Document

- **When discussing trades**: Use "operation point", not "trade" or "signal". Specify "long" or "short".
- **When discussing market data**: Use "resampled point" or "D1 candle", not "price bar" or "OHLC".
- **When discussing risk rules**: Use "money management strategy" (MMS), not "position sizing" or "risk model".
- **When discussing validation**: Reference the specific invariant by name (e.g. "same-instrument validation").
- **When discussing cache**: Use "signal-group memoization". The `cached` crate memoizes per `(indicator_id, instrument)` in-process.
- **When discussing strategies**: Use "strategy" (a configured indicator + op-point sets + MMS + metrics). Reference "annual ROI", "max draw down", "annual operation count" by name.
- **When discussing batch runs**: Use "cell" for one (instrument, mms_id) pair; "manifest" for the processed ledger.
- **When adding new entities**: Follow the pattern — struct + repository trait in domain, adapter in infrastructure, fixtures in `domain::fixtures`.
- **When adding new metrics**: Follow `annual_operation_count` (free function) or `performance_metrics` (struct-returning single-pass fn + method) — pure function in `domain/`, unit-tested in the same file. Curve-coupled metrics belong together in one pass (see `performance_metrics.rs`); standalone metrics get their own file.
- **When adding new indicators**: Add an `IndicatorParameters` variant, a compute function in `domain/indicators/` (use `validate_period!` / `empty_oscillator!` / `indicator_values!` macros), a dispatch arm in `create_indicator_output`, and tests. See `rsi.rs` / `bears.rs`.
