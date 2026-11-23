<!-- Context: project-intelligence/domain-model | Priority: critical | Version: 1.19 | Updated: 2026-11-14 -->

# Domain Model — Backtesting Engine

> Ubiquitous language, entity definitions, invariants, and validation rules extracted from the Rust codebase.
> This is the single source of truth for how we talk about the domain.

## Ubiquitous Language (Glossary)

| Term | Definition | Rust Type | Module |
|------|-----------|-----------|--------|
| **Instrument** | A tradeable financial pair (e.g. `EURUSD`, `USDCAD`). Identifies what is being traded. | `String` field | multiple |
| **Operation Point** | A single trade entry — a point in time where a position was opened. Contains the trade's result, take-profit, stop-loss, risk, and the money management strategy used. An operation point stamped T is entered at the **open** of resampled point T. | `OperationPoint` | `domain::operation_point` |
| **Operation Points (bundle)** | A paired collection of long and short operation points. Provides timestamp-keyed lookup maps for aligning trades with market data. | `OperationPoints` | `domain::operation_point` |
| **Long Operation Point** | An operation point representing a **buy** (long) position. Stored in a separate repository from shorts. | `OperationPoint` via `LongOperationPointRepository` | `domain::operation_point` |
| **Short Operation Point** | An operation point representing a **sell** (short) position. Stored in a separate repository from longs. | `OperationPoint` via `ShortOperationPointRepository` | `domain::operation_point` |
| **Resampled Point (D1)** | An OHLCV candlestick bar resampled to the **daily (D1)** timeframe. | `ResampledPointD1` | `domain::resampled_point` |
| **Indicator** | A technical indicator applied to market data. Has an `id` and typed `parameters`. | `Indicator` | `domain::indicator` |
| **Indicator Parameters** | Typed parameters for an indicator — a 20-variant enum: `Macd`, `Rsi`, `AwesomeOscillator`, `AcceleratorDecelerator`, `BearsPower`, `BullsPower`, `Cci`, `ChaikinOscillator`, `ChaikinVolatility`, `Dema`, `DetrendedPriceOscillator`, `ForceIndex`, `Mfi`, `OsMa`, `Roc`, `Rvi`, `Stochastic`, `Tema`, `Trix`, `Vroc`. | `IndicatorParameters` | `domain::indicator` |
| **Moving Average Type** | The moving average behind one MACD line: `Ema` or `Sma`. The PG adapter maps the companion's `"ema"`/`"sma"` strings to it and rejects any other value when loading. | `MovingAverageType` | `domain::indicator` |
| **Indicator Type** | The category/kind of a technical indicator (e.g. `macd`, `rsi`). Not a domain-struct field: lives in the database `type` column, which the PG adapter reads to select the `IndicatorParameters` variant. | `type` DB column | `infrastructure::pg_adapter` |
| **Indicator Output** | The computed result of an indicator over a candle series — `Oscillator`, `Threshold`, or `Crossover` — carrying timestamped values. Converted to a `SignalGroup` by signal semantics. | `IndicatorOutput` | `domain::signal` |
| **Money Management Strategy (MMS)** | A strategy that governs position sizing and risk. Identified by `money_management_strategy_id`. All operation points in a run must use the same MMS. | `i32` field | `domain::operation_point`, `domain::strategy` |
| **Take Profit (TP)** | Take-profit distance in integer price units, the same units as SL. Loaded from the source; not read by any computation yet (reserved for exit-date modeling). | `i32` field (`tp`) | `domain::operation_point` |
| **Stop Loss (SL)** | Stop-loss distance in integer price units. Always positive: the PG adapter rejects `sl <= 0`, because the metrics divide by it. | `i32` field (`sl`) | `domain::operation_point` |
| **Risk** | The fraction of capital risked on a single trade (e.g. `0.01` = 1%). | `f64` field | `domain::operation_point` |
| **Result** | Realized outcome of a trade in the same integer units as SL: positive for a gain, negative for a loss. The per-trade return on the curve is `result / sl`. | `i32` field | `domain::operation_point` |
| **Timestamp** | An integer epoch (seconds) identifying when a data point or trade occurred. Used to align operation points with resampled market data. | `i64` field | multiple |
| **Strategy** | A backtested configuration: one indicator + a set of long/short operation point IDs + a money management strategy, annotated with performance metrics. Carries its `instrument`: unread by the engine and not a stored column (the partition path holds it), kept on purpose for tracing. | `Strategy` | `domain::strategy` |
| **Annual ROI** | Annualized return on investment for a strategy. Computed by compounding the global ROI over the time span; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::performance_metrics` (method) |
| **Max Draw Down** | Largest peak-to-trough decline of the cumulative-return curve, as a percentage. Computed from operation points via compounding against a running cummax; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::performance_metrics` |
| **Annual Operation Count** | Number of trades per year, annualized from a total count over the time span. Computed from signal-group operation counts; stored on `Strategy`. | `f64` field | `domain::strategy`, `domain::annual_operation_count` |
| **Global Strategy Filter** | Default thresholds for keeping a strategy: max draw down ≤ 20%, min annual ROI ≥ 5%. Annual operation count ≥ 10, checked on matched trades (the stored count). Owns the threshold checks: `admits_activity()` and `admits_performance()`. | `GlobalStrategyFilter` | `domain::strategy` |
| **Signal Group** | A collection of long and short signal timestamps for a single indicator. Embeds the `indicator_id` and `instrument` it belongs to, plus a `signal_count()` derived from its signals. The `instrument` only travels on to `Strategy` for tracing; the signal cache does not key on it. Lives in the domain layer. A cross read at the close of resampled point T is stored as the timestamp of the **next** resampled point (signals on the last one are dropped), so it selects the operation point entered after T closed. | `SignalGroup` | `domain::signal_group` |
| **Cell** | One `(instrument, mms_id)` pair — the unit of work for `create_loop`. Cells are computed in parallel and persisted as one Parquet partition each. | `Cell`, `CellOutput` | `domain::cell`, `application::cell_output`, `application::strategy_batch` |
| **Parquet Strategy Repository** | Writes strategies to partitioned parquet files under `{base}/instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. Dedups fresh rows by `indicator_id` (idempotent partitions). | `ParquetStrategyRepository` | `infrastructure::strategy_adapter` |
| **Processed Manifest** | Parquet ledger of completed `(instrument, mms_id, indicator_id)` triples with `processed_at` timestamps. Makes `create`/`create_loop` idempotent. Atomic writes (temp + rename); dedup per triple. | `ParquetProcessedManifest` | `infrastructure::processed_manifest` |
| **Strategy Persistence** | Batch commit for `create_loop`: assigns `strategy_id`s from the global max, writes partitions in parallel (rayon), commits the manifest once. | `StrategyPersistence` | `infrastructure::strategy_persistence` |
| **Strategy Runner** | The core use case: validate → signal groups → filter by annual ops → create strategies (real metrics) → filter by DD/ROI. | `run_strategies` | `application::strategy_runner` |
| **Strategy Generator** | Builds `Strategy` records from signal groups + operation points in one rayon pass: drops groups below the minimum matched annual operation count, computes real metrics (`annual_roi`, `max_draw_down`, `annual_operation_count`) for the rest, and keeps only those passing the performance check. | `create_strategies` | `application::strategy_generator` |
| **Source Data** | The fetched-from-PostgreSQL inputs for strategy generation: indicators, resampled points, long/short operation points for one (instrument, mms_id). | `SourceData`, `fetch_source_data` | `application::source_data` |
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
    result:                      i32       — realized outcome, same units as sl (positive = gain)
    tp:                          i32       — take-profit distance (reserved; not read yet)
    sl:                          i32       — stop-loss distance (always > 0)
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
//          start_end_timestamps()      -> Option<(i64, i64)>  — (min, max) of long op timestamps; None when empty
// Duplicate timestamps: last wins.
```

### ResampledPointD1
Daily OHLCV candlestick bar — the market data backdrop against which trades are evaluated.
```
ResampledPointD1 {
    id:              i32       — DB primary key; unread by the engine, kept on purpose for tracing rows back to the DB
    instrument:      String    — e.g. "EURUSD"
    open:            f64       — opening price
    high:            f64       — highest price
    low:             f64       — lowest price
    close:           f64       — closing price
    volume:          i32       — volume as supplied by the source (used by CHO, Force, VROC)
    timestamp:       i64       — day identifier (epoch seconds)
}
```

### Indicator
A technical indicator used in the strategy being backtested.
```
Indicator {
    id:          i32                 — unique identifier
    parameters:  IndicatorParameters — 20-variant typed enum (see below)
}
IndicatorParameters ::= Macd { slow_type: MovingAverageType, slow_n, fast_type: MovingAverageType, fast_n }
                      | Rsi { n, high_threshold, low_threshold }
                      | AwesomeOscillator
                      | AcceleratorDecelerator
                      | BearsPower { period }
                      | BullsPower { period }
                      | Cci { period }
                      | ChaikinOscillator { fast_ma, slow_ma }
                      | ChaikinVolatility { smooth_period, chv_period }
                      | Dema { period }
                      | DetrendedPriceOscillator { period }
                      | ForceIndex { period }
                      | Mfi { period, high_threshold, low_threshold }
                      | OsMa { fast_ema, slow_ema, signal_sma }
                      | Roc { period }
                      | Rvi { period }
                      | Stochastic { k_period, slowing, high_threshold, low_threshold }
                      | Tema { period }
                      | Trix { period }
                      | Vroc { period }
MovingAverageType   ::= Ema | Sma
```

### IndicatorOutput (domain::signal)
The computed result of an indicator over a candle series, and its signal semantics:
```
IndicatorOutput ::= Oscillator { instrument, buffer: OscillatorBuffer }
                  | Threshold  { instrument, buffer: ThresholdBuffer }
                  | Crossover  { instrument, buffers: CrossoverBuffers }

IndicatorValue { timestamp: i64, value: f64 }
OscillatorBuffer { items: Vec<IndicatorValue> }
ThresholdBuffer  { items, high_threshold, low_threshold }
CrossoverBuffers    { upcross_long_buffer, upcross_short_buffer }

// IndicatorOutput::create_signal_group(indicator_id) -> SignalGroup
// Signal rules (domain/signal/):
//   Values exactly on a level are skipped, so a cross through it (-1, 0, 1) still
//   fires, on the first bar past the level; a touch-and-bounce (1, 0, 1) does not.
//   Oscillator: prev < 0 && curr > 0 -> long; prev > 0 && curr < 0 -> short (zero-cross)
//   Threshold:  prev < low && curr > low -> long; prev > high && curr < high -> short
//   Crossover:  (long_buffer - short_buffer) diff, then zero-cross
```

### Strategy
A backtested configuration annotated with performance metrics.
```
Strategy {
    instrument:                     String    — which instrument this strategy ran on (kept for tracing; nothing reads it)
    indicator_id:                   i32       — which indicator this strategy uses
    annual_roi:                     f64       — annualized return on investment (%)
    max_draw_down:                  f64       — largest peak-to-trough decline (%)
    annual_operation_count:         f64       — trades per year (annualized)
    long_operation_point_ids:       Vec<i32>  — long trades in this strategy
    short_operation_point_ids:      Vec<i32>  — short trades in this strategy
    money_management_strategy_id:   i32       — which MMS governs this strategy
}
// Note: strategy_id is NOT a domain field. IDs are assigned at persistence time
// by StrategyPersistence::persist_cells (global max + 1 across all partitions).
```

### GlobalStrategyFilter
The single home of strategy thresholds and their checks. `Default`: max_draw_down_max=20.0, min_annual_roi=5.0, min_annual_operation_count=10.0.
```
GlobalStrategyFilter { max_draw_down_max: f64, min_annual_roi: f64, min_annual_operation_count: f64 }
// Method: admits_activity(annual_operation_count) -> bool — count >= min
// Method: admits_performance(annual_roi, max_draw_down) -> bool — ROI >= min && DD <= max
```

### SignalGroup
A group of signal timestamps (long and short) for a single indicator. Embeds its `indicator_id` and `instrument`, plus a `signal_count()` derived from its signals. Lives in the domain layer.
```
SignalGroup {
    indicator_id:     i32       — which indicator this group belongs to
    long_signals:     Vec<i64>  — long signal timestamps
    short_signals:    Vec<i64>  — short signal timestamps
    instrument:       String    — which instrument the signals were computed on (only carried on to Strategy)
}
// Method: signal_count() -> usize — long.len() + short.len()
// Free fn: filter_signal_groups_by_annual_ops(groups, &filter, start_ts, end_ts) -> Vec<SignalGroup>
//          Keeps groups with > 0 signals whose annualized signal count passes filter.admits_activity().
```

---

## Domain Metric Functions (Pure)

Pure functions in `domain/` — no I/O, no frameworks. `performance_metrics()` returns a `PerformanceMetrics` struct (both curve-derived metrics in one pass); the others are free functions over plain values.

| Function | File | Signature | Behavior |
|----------|------|-----------|----------|
| `annual_operation_count` | `domain::annual_operation_count` | `(total_count: usize, start_ts: i64, end_ts: i64) -> f64` | `total_count / years`, where `years = (end-start)/86400/365.25` (not rounded). The result is rounded to 2dp. Returns 0.0 for zero count or non-positive span. |
| `performance_metrics` | `domain::performance_metrics` | `(ops: Vec<&OperationPoint>) -> PerformanceMetrics` | Sorts the owned ops chronologically in place (stable; ties keep input order), then ONE pass: `cumsum = 1 + Σ(cummax * result * risk/sl)` with running `cummax`. Returns `{ global_roi: (cumsum - 1) * 100, max_draw_down: deepest (cummax - cumsum)/cummax * 100 }`, each rounded to 2dp independently. Both 0.0 for empty ops. |
| `PerformanceMetrics::annual_roi` | `domain::performance_metrics` | `(&self, start_ts: i64, end_ts: i64) -> f64` | Annualizes `self.global_roi` via compounding: `((1 + roi/100)^(1/years) - 1) * 100`, rounded to 2dp. Returns 0.0 for non-positive span. |
| `filter_signal_groups_by_annual_ops` | `domain::signal_group` | `(groups: Vec<SignalGroup>, filter: &GlobalStrategyFilter, start_ts: i64, end_ts: i64) -> Vec<SignalGroup>` | Keeps groups with `signal_count() > 0` and `filter.admits_activity(annual_operation_count(signal_count, start, end))`. Raw signals are an upper bound on matched trades, so this never drops a passing group. |
| `unprocessed_indicators` | `domain::indicator` | `(indicators: &[Indicator], processed_ids: &FxHashSet<i32>) -> Vec<Indicator>` | Set-diff of all indicators vs manifest-processed IDs, keeping input order. `indicator_ids` lists the IDs of a slice. |

---

## Indicator Computation (domain::indicators)

`create_indicator_output(parameters, points) -> Result<IndicatorOutput, IndicatorError>` dispatches on the typed parameter enum:

| Indicator | Output kind | Notes |
|-----------|-------------|-------|
| Rsi | Threshold | over close prices |
| Mfi | Threshold | volume-weighted RSI over the typical price `(high + low + close) / 3` (`mfi.rs`); a window with no negative money flow reads 100, as in MQL5. |
| Stochastic | Threshold | %K: the close within the `k_period` high-low range, smoothed over `slowing` ranges (`stochastic.rs`); a window with no range reads 100, as in MQL5. The %D signal line is not computed. |
| AwesomeOscillator, AcceleratorDecelerator, BearsPower, BullsPower, Cci, ChaikinOscillator, ChaikinVolatility, DetrendedPriceOscillator, ForceIndex, OsMa, Roc, Trix, Vroc | Oscillator | one file each in `domain/indicators/` |
| Macd | Crossover | MACD line (fast MA minus slow MA, each EMA or SMA) crossed against its signal line `SMA(MACD, 9)` (`macd.rs`). The signal period is an engine constant, not a database parameter. |
| Dema | Crossover | the close crossed against `2*EMA(close) - EMA(EMA(close))` (`dema.rs`): DEMA is a price overlay, so it has no zero line to cross. |
| Rvi | Crossover | RVI line `sum(body) / sum(range)` over `period` bars, each bar's body `close - open` and range `high - low` first smoothed 1-2-2-1 over four bars, crossed against its signal line, the same 1-2-2-1 average of the RVI line (`rvi.rs`). |
| Tema | Crossover | the close crossed against `3*EMA(close) - 3*EMA(EMA(close)) + EMA(EMA(EMA(close)))` (`tema.rs`): like DEMA, a price overlay with no zero line to cross. |

- `IndicatorError` has typed per-indicator period variants and `NotEnoughData`.
- Period validation is uniform via the `validate_period!` macro (non-positive period → typed error).
- Insufficient data yields an **empty buffer**, not an error (`empty_oscillator!` macro; the MACD, DEMA and TEMA crossover buffers use `empty_crossover!`).
- Shared moving-average math lives in `domain/indicators/moving_averages.rs`.
- `PriceField` (Open/High/Low/Close) selects which candle component an indicator consumes.

---

## Repository Traits (Ports)

The domain defines repository traits — ports in a ports-and-adapters architecture. Every method takes `&mut self`: a read uses the adapter's PostgreSQL connection.

| Trait | Methods | Semantics |
|-------|---------|-----------|
| `LongOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)`, `index_by_cell()` (default) | Long (buy) trades; `index_by_cell` groups by `Cell` |
| `ShortOperationPointRepository` | `get_all()`, `get_by_mms_id_and_instrument(mms_id, instrument)`, `index_by_cell()` (default) | Short (sell) trades |
| `ResampledPointD1Repository` | `get_all()`, `get_by_instrument(instrument)`, `index_by_instrument()` (default) | Daily candles |
| `IndicatorRepository` | `get_all()` | Indicators |

Infrastructure provides one PostgreSQL implementation: `PgStore` implements all four traits. The in-memory test repos were removed (2026-08); tests use `domain::fixtures` builders + `tempfile`-based adapter tests instead. There is no dedicated money-management-strategy adapter: `create_loop` derives its cell universe from the operation-point indexes themselves (`index_by_cell()`), and the long and short tables carry the same money-management id set (written together by the upstream Backtesting-4.0 project).

`PgStore` exposes no public read methods beyond the repository traits; `PgStore::connect(&DbConfig)` opens one connection.

---

## Application Layer

`src/application/` — orchestration between domain and infrastructure. One file per use case.

| Unit | File | Purpose |
|------|------|---------|
| `fetch_source_data` | `application::source_data` | Takes `&DbConfig` (the `engine` handlers resolve the environment), opens one `PgStore`, and fetches indicators/resampled/long/short for one (instrument, mms_id). `PgStore::connect` adds connection diagnostics on failure. |
| `create_signal_groups` | `application::signal_generator` | Computes REAL indicator outputs (`create_indicator_output`) and converts them to `SignalGroup`s via signal semantics. Rayon `par_iter`; per-indicator memoization via `#[cached]` keyed `(indicator_id, instrument)`. Indicators that fail to compute are logged as warnings and skipped, not aborted. |
| `create_strategies` | `application::strategy_generator` | One **rayon** `par_iter()` pass per signal group: counts matched trades and drops the group unless `admits_activity()`; builds the `Strategy` with real metrics; keeps it only if `admits_performance()`. Survivors keep input order. |
| `run_strategies` | `application::strategy_runner` | The core use case: `validate_all` → `create_signal_groups` → `filter_signal_groups_by_annual_ops` → `create_strategies` (matched-trade and performance filters inside). |
| `SourceIndex`, `compute_cells`, `fail_on_total_failure` | `application::strategy_batch` | The `create_loop` batch use case: loads all four source tables once (parallel `rayon::join`), computes every cell with `par_iter`, summarises outcomes into `BatchSummary`, and flags a total failure (no cell computed, at least one failed) as systemic. |
| `CellOutput` | `application::cell_output` | One cell's compute result: the `Cell`, its strategies, and the computed indicator IDs. |

---

## Infrastructure Adapters

| Adapter | Purpose | File |
|---------|---------|------|
| `PgStore` | PostgreSQL reads; implements all four repository traits | `src/infrastructure/pg_adapter.rs` |
| `ParquetStrategyRepository` | Writes strategies to partitioned parquet files (ZSTD compression) | `src/infrastructure/strategy_adapter.rs` |
| `ParquetProcessedManifest` | Idempotency ledger of processed (instrument, mms_id, indicator_id) triples | `src/infrastructure/processed_manifest.rs` |
| `StrategyPersistence` | Batch commit for create_loop: ID assignment + parallel partition writes + manifest commit | `src/infrastructure/strategy_persistence.rs` |
| `logging` | simplelog initialization (term + file, RUST_LOG-driven) | `src/infrastructure/logging.rs` |

### Parquet Strategy Repository
Partition layout: `{base_dir}/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet`
- Schema: `strategy_id` (Int64), `indicator_id` (Int32), `annual_roi`/`max_draw_down`/`annual_operation_count` (Decimal128(10,2)), `long_operation_point_ids`/`short_operation_point_ids` (List<Int32>), `money_management_strategy_id` (Int32).
- `base_dir` is `{parquet_root}/strategy`, derived by `StrategyPersistence::new(parquet_root)`; `parquet_root` resolved by `config::environment::parquet_root_dir(env)` → `/app/parquet_files/{env}`.
- **Idempotent partitions**: `write_partition` reads existing rows and drops fresh rows whose `indicator_id` already exists — re-runs never duplicate.
- **Append semantics**: write = read existing + append fresh + rewrite the file in place. Rows arrive with their `strategy_id` already set: `StrategyPersistence::persist_cells` assigns `max_strategy_id() + 1` onwards (global max across ALL partitions, scanned recursively). IDs are contiguous within a batch and continue across partitions and appends.
- API: `write_rows(&Cell, Vec<StrategyRow>)` (rows carry their IDs), `max_strategy_id()`. `StrategyRow::from_strategy(Strategy, id)` builds a row.

### Processed Manifest
`{root_dir}/processed.parquet` — schema: `instrument` (Utf8), `mms_id` (Int32), `indicator_id` (Int32), `processed_at` (Int64).
- `read_processed_ids_per_cell()` (keyed by `Cell`), `append_all(&[ManifestEntry], processed_at)`. A `ManifestEntry` holds a `Cell` and its indicator IDs.
- Dedups on the (instrument, mms_id, indicator_id) triple; atomic writes (temp file + fsync + rename; temp file removed on failure).
- Consumed by `strategy create` (skip processed) and `create_loop` (per-cell skip).
- **Not tracked:** the engine's logic (signal, indicator, metric and validation rules), the `GlobalStrategyFilter` defaults, and the source data in PostgreSQL. "Idempotent" means "never computed twice", not "up to date": after any of those changes, a listed cell still holds its stale strategies. The manifest and the partitions must be reset together, by deleting the whole `/app/parquet_files/{env}` directory and running `create_loop`; strategy IDs then restart at 1, and only that environment is reset. Deleting only the manifest keeps the old rows (partitions dedup by `indicator_id`); deleting only the partitions recomputes nothing. See the living notes gotcha.

---

## Validation Rules (Domain Invariants)

Before a backtest can run, the input data must satisfy **five invariants**, checked in order by `validate_all()`:

1. **Non-Empty Data** (`empty_validation`) — all four datasets non-empty.
2. **Same Instrument** (`same_instrument_validation`) — every resampled/long/short point references the same instrument (itertools `all_equal_value()` over the chained iterators; the error names the first two differing instruments).
3. **Same Money Management Strategy** (`same_mms_validation`) — every operation point's `money_management_strategy_id` matches the expected MMS ID parameter.
4. **Timestamp Alignment** (`same_timestamp_validation`) — long and short timestamp sets are identical; both are subsets of resampled timestamps.
5. **Chronological Resampled Points** (`chronological_order_validation`) — resampled point timestamps are strictly increasing, so an unsorted series and a repeated timestamp are both rejected; the error names the first offending pair. Indicators and the next-resampled-point entry search (`partition_point`) rely on this order.

`combined_validation::validate_all()` runs all five in order, returning early on the first failure as a `validation::ValidationError` (LongOperationPointsEmpty / ShortOperationPointsEmpty / IndicatorsEmpty / ResampledPointsEmpty / InstrumentMismatch / MoneyManagementStrategyMismatch / LongShortTimestampsDiffer / TimestampMissingFromResampled / ResampledPointsNotChronological). Order is load-bearing: emptiness first — otherwise the same-X checks would vacuously pass on empty input.

---

## Architecture Layers

```
src/
├── application/                # Orchestration between domain & infrastructure
│   ├── mod.rs
│   ├── cell_output.rs          # CellOutput — one (instrument, mms_id) cell's compute result
│   ├── signal_generator.rs     # create_signal_groups() — REAL indicator-derived signals
│   ├── source_data.rs          # fetch_source_data() — PG-backed data assembly
│   ├── strategy_batch.rs       # SourceIndex + compute_cells() — create_loop batch use case
│   ├── strategy_generator.rs   # create_strategies() / create_strategy() — staged filters + real metrics
│   └── strategy_runner.rs      # run_strategies() — validate → signals → filter → strategies
├── domain/                     # Pure domain logic — no I/O, no frameworks
│   ├── cast.rs                 # the only numeric `as` casts + metric rounding (round_to)
│   ├── cell.rs                 # Cell — the (instrument, mms_id) unit of work; Display + Ord
│   ├── indicator.rs            # Indicator + IndicatorParameters (20 variants) + IndicatorRepository + unprocessed_indicators()
│   ├── indicators/             # Indicator computation: 20 indicators + dispatch + macros + moving averages
│   │   ├── mod.rs              # create_indicator_output() dispatch, PriceField, IndicatorError
│   │   ├── macros.rs           # indicator_values!, validate_period!, empty_oscillator!, empty_crossover!
│   │   ├── moving_averages.rs  # shared MA math
│   │   ├── rsi.rs, ao.rs, ac.rs, bears.rs, bulls.rs, cci.rs, cho.rs, chv.rs,
│   │   ├── dema.rs, dpo.rs, force.rs, mfi.rs, osma.rs, roc.rs, rvi.rs, stochastic.rs, tema.rs, trix.rs, vroc.rs
│   │   └── test_support.rs     # #[cfg(test)] shared test helpers
│   ├── operation_point.rs      # OperationPoint + OperationPoints + long/short repo traits (+ index_by_cell)
│   ├── resampled_point.rs      # ResampledPointD1 + repo trait (+ index_by_instrument)
│   ├── signal/                 # Signal semantics: IndicatorOutput → SignalGroup
│   │   ├── mod.rs              # IndicatorOutput enum + create_signal_group()
│   │   ├── oscillator.rs       # zero-cross signals
│   │   ├── thresholds.rs       # threshold-crossing signals
│   │   └── crossover.rs        # diff-of-buffers → zero-cross signals
│   ├── signal_group.rs         # SignalGroup + filter_signal_groups_by_annual_ops()
│   ├── strategy.rs             # Strategy + GlobalStrategyFilter (admits_activity / admits_performance)
│   ├── annual_operation_count.rs  # annual_operation_count() metric
│   ├── performance_metrics.rs  # performance_metrics() -> PerformanceMetrics (single pass) + annual_roi() method
│   ├── time_span.rs            # difference_in_years() shared by annualised metrics
│   ├── validation/             # Domain invariant checks (5 rules + combined)
│   └── fixtures.rs             # #[cfg(test)] test data builders
├── engine/                     # The Rust CLI (`bte`); handlers resolve the environment
│   ├── mod.rs                  # EngineCommand enum, build_cli(), parse(), execute()
│   └── strategy/               # bte strategy commands
│       ├── mod.rs              # StrategyCommand enum (create, create_loop)
│       ├── create.rs           # strategy create — single cell pipeline
│       └── create_loop.rs      # strategy create_loop — CLI handler: load, compute, persist, report
├── infrastructure/             # Adapters (PG, parquet, manifest, logging)
│   ├── pg_adapter.rs           # PgStore — implements all four domain repo traits
│   ├── strategy_adapter.rs     # ParquetStrategyRepository (partitioned, idempotent parquet writes)
│   ├── processed_manifest.rs   # ParquetProcessedManifest (idempotency ledger)
│   ├── parquet_columns.rs      # checked column reads: a mistyped file is an error, never a panic
│   ├── strategy_persistence.rs # StrategyPersistence (batch commit)
│   └── logging.rs              # simplelog init
├── config/                     # Environment + DB configuration
│   └── environment.rs          # Environment, DbConfig, resolve_environment(), parquet_root_dir()
├── bin/
│   └── bte.rs            # bte binary entry: engine::build_cli() → parse() → execute()
└── lib.rs                      # Crate root — re-exports application, config, domain, engine, infrastructure
```

---

## Key Design Decisions Visible in Code

1. **Long/Short separation**: Longs and shorts use the same `OperationPoint` struct but are stored in separate repositories. Direction is determined by *which repository* holds the point, not by a field on the struct.
2. **Validation as pure functions**: All validators are free functions over slices, not methods on structs. Composable and testable without object construction.
3. **Fail-fast combined validation**: `validate_all` short-circuits on the first error. Order matters: emptiness is checked first — otherwise the same-X checks would vacuously pass on empty input.
4. **Fixtures module**: Test data lives in `domain::fixtures`, reusable across all test modules. (In-memory repos were removed 2026-08 — fixtures + tempfile cover the same need with less code.)
5. **Ports-and-adapters**: Repository traits live in the domain; implementations live in infrastructure. The domain has zero dependencies on infrastructure.
6. **Single-binary engine layout**: `bte` (`src/bin/bte.rs`) is the sole Rust CLI; it follows the `build -> parse -> execute` pattern and returns `anyhow::Result<()>`. The previous `ct` operator frontend (which forwarded engine commands via Docker exec and Python CLI commands via the `api` container) was removed in the commit that dropped `src/cli/` and `src/main.rs` — data CRUD and the iteration table are now reached exclusively in the companion Python service.
7. **Application layer**: `src/application/` orchestrates domain + infrastructure. Keeps `engine/` CLI handlers thin. One file per use case.
8. **Metrics as pure domain functions**: `performance_metrics()` takes an owned `Vec<&OperationPoint>` (sorted in place, no copy) and returns `PerformanceMetrics` (`global_roi` + `max_draw_down`) computed in a single pass over the shared compounding curve, sorting its input chronologically itself (stable; ties keep input order). `annual_roi` is a method on `PerformanceMetrics`. No I/O — composable and unit-tested in isolation.
9. **Staged filtering**: each check runs at the earliest stage where its input exists. (1) `filter_signal_groups_by_annual_ops()` on raw signal counts (free, upper bound); (2) inside `create_strategies`, on matched-trade counts (hash lookups, no allocation), before any metrics; (3) inside the same parallel pass, on max draw down + annual ROI (expensive). All thresholds and checks live on `GlobalStrategyFilter` (`admits_activity`, `admits_performance`); `GlobalStrategyFilter::default()` encodes the baseline quality bar (DD ≤ 20%, ROI ≥ 5%, ops ≥ 10/yr).
10. **Parquet partitioning by (instrument, mms_id)**: Strategies are written as partitioned parquet files — `instrument=.../money_management_strategy_id=.../strategies.parquet`. Matches the single-instrument, single-MMS backtest constraint and enables efficient per-partition reads.
11. **Typed indicator parameters**: `IndicatorParameters` is a 20-variant enum — the PG adapter deserializes the JSON `parameters` column with `serde_json`, selecting the variant by the `type` column.
12. **Real metrics in the generator**: `create_strategies()` computes `performance_metrics()` → `annual_roi()`, plus `annual_operation_count`, from each strategy's referenced operation points. Parallelized with **rayon** `par_iter()`.
13. **`SignalGroup` lives in the domain**: `SignalGroup` embeds `indicator_id`, `instrument`, and a derived `signal_count()`. The cache stores `Vec<SignalGroup>` — IDs travel with each group. The `instrument` chain (`IndicatorOutput` → `SignalGroup` → `Strategy`) is kept on purpose for tracing, although no production code reads `Strategy::instrument`.
14. **Timestamps are `i64`**: All timestamps are `i64` epoch seconds. The PG adapter casts `i32` DB columns to `i64` on read.
15. **`strategy_id` is persistence-assigned, not domain-owned**: The `Strategy` struct has no `strategy_id` field. `StrategyPersistence::persist_cells` assigns IDs before the parquet write (global max across all partitions + 1). Keeps the domain free of persistence concerns.
16. **Idempotent parquet partitions**: `write_partition` dedups fresh rows by `indicator_id` against existing rows — re-runs never duplicate strategies for the same indicator.
17. **Processed manifest for idempotency**: `ParquetProcessedManifest` records completed (instrument, mms_id, indicator_id) triples; `create`/`create_loop` only compute the unprocessed remainder. Atomic writes (temp + rename).
18. **Real signal generation**: `create_signal_groups()` computes actual indicator outputs (`create_indicator_output`) and derives signals via zero-cross / threshold / crossover semantics. All 20 indicators, MACD, DEMA, RVI and TEMA included, compute real outputs.
19. **Batch parallelism in `create_loop`**: All four source tables loaded once via nested `rayon::join`, indexed in memory; cells computed via `par_iter`; persistence assigns IDs once, writes partitions in parallel, commits the manifest once.
20. **Entry timing (no look-ahead)**: indicator values at resampled point T use T's data, known only at its close, while the operation point stamped T is entered at T's open. `IndicatorOutput::create_signal_group` therefore shifts every signal to the next resampled point's timestamp before building the `SignalGroup`; `create_strategy` then matches signal and operation point timestamps as is.

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
