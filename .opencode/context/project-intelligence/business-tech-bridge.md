<!-- Context: project-intelligence/bridge | Priority: high | Version: 1.12 | Updated: 2026-11-02 -->

# Business ↔ Tech Bridge

> How domain concepts map to code. Use this to navigate from a business question to the right file.

## Core Mapping

| Business Concept | Rust Implementation | File | Notes |
|-----------------|---------------------|------|-------|
| A trade (long or short) | `OperationPoint` struct | `src/domain/operation_point.rs` | Direction determined by repository, not a field |
| Long/short trade bundle | `OperationPoints` struct | `src/domain/operation_point.rs` | Pairs long+short; `*_points_by_timestamp()` lookup maps |
| Long trades collection | `LongOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure; `index_by_cell()` default method |
| Short trades collection | `ShortOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure; `index_by_cell()` default method |
| Daily market candle | `ResampledPointD1` struct | `src/domain/resampled_point.rs` | OHLCV + instrument + timestamp |
| Market data collection | `ResampledPointD1Repository` trait | `src/domain/resampled_point.rs` | Port — `get_all()`, `get_by_instrument()`, `index_by_instrument()` |
| Technical indicator | `Indicator` struct | `src/domain/indicator.rs` | Has `id` and typed `parameters` |
| Indicator parameters | `IndicatorParameters` enum | `src/domain/indicator.rs` | 14 variants: Macd, Rsi, AO, AC, Bears, Bulls, CHO, CHV, DPO, Force, OsMa, Roc, Trix, Vroc |
| Indicator computation | `create_indicator_output()` | `src/domain/indicators/mod.rs` | Dispatches params → `IndicatorOutput`; all 14 indicators compute (MACD as a crossover of its line and signal line) |
| Indicator math (one per indicator) | `rsi.rs`, `ao.rs`, … | `src/domain/indicators/` | Shared MAs in `moving_averages.rs`; macros in `macros.rs` |
| Indicator output → signals | `IndicatorOutput::create_signal_group()` | `src/domain/signal/mod.rs` | Oscillator/threshold/crossover semantics |
| Money management strategy | `money_management_strategy_id` field | `src/domain/operation_point.rs`, `src/domain/strategy.rs` | Integer ID, no dedicated struct yet |
| A backtested strategy | `Strategy` struct | `src/domain/strategy.rs` | instrument + indicator + op-point ID sets + MMS + metrics |
| Strategy quality bar | `GlobalStrategyFilter` + `filter_strategies()` | `src/domain/strategy.rs` | Default: DD≤20%, ROI≥5% (op count filtered earlier) |
| Annual return on investment | `annual_roi` field on `Strategy` | `src/domain/strategy.rs` | Computed by `PerformanceMetrics::annual_roi()` from global ROI + time span |
| "How deep did it draw down?" / "What is the total return?" | `performance_metrics()` → `.max_draw_down` / `.global_roi` | `src/domain/performance_metrics.rs` | Single-pass fn over `&[&OperationPoint]` (sorts chronologically itself), returns % |
| "How many trades per year?" | `annual_operation_count()` | `src/domain/annual_operation_count.rs` | Pure fn over `(count: usize, start_ts: i64, end_ts: i64)` |
| "Is the data valid?" | `validate_all()` | `src/domain/validation/combined_validation.rs` | Runs 4 checks, fails fast |
| "Data must not be empty" | `validate_empty_entities()` | `src/domain/validation/empty_validation.rs` | Checks all 4 datasets |
| "All same instrument" | `validate_same_instrument()` | `src/domain/validation/same_instrument_validation.rs` | All resampled and trade points share one instrument |
| "All same MMS" | `validate_money_management_strategy_id_matches()` | `src/domain/validation/same_mms_validation.rs` | Checks against expected ID param |
| "Timestamps align" | `validate_timestamps_match()` | `src/domain/validation/same_timestamp_validation.rs` | Long and short timestamps are identical sets, contained in resampled timestamps |
| Test data | `domain::fixtures::*` functions | `src/domain/fixtures.rs` | Shared across all test modules |
| Environment config | `Environment` enum + `DbConfig` + `resolve_environment()` | `src/config/environment.rs` | Single `ENVIRONMENT` resolver: absent → development, invalid → fail-fast error |
| Parquet base directory | `parquet_base_dir(env)` / `parquet_root_dir(env)` | `src/config/environment.rs` | `/app/parquet_files/{env}/strategy` and `/app/parquet_files/{env}` |
| Fetch strategy inputs from DB | `fetch_source_data(query)` | `src/application/source_data.rs` | `(instrument, mms_id)` → `SourceData` |
| Generate signal groups from indicators | `create_signals` → **`create_signal_groups`** | `src/application/signal_generator.rs` | REAL indicator outputs → signal semantics; rayon + `#[cached]` keyed `(indicator_id, instrument)` |
| Build strategies from signal groups | `create_strategies()` / `create_strategy()` | `src/application/strategy_generator.rs` | One strategy per signal group; computes real metrics; rayon-parallelized |
| Run the full pipeline for one cell | `run_strategies()` | `src/application/strategy_runner.rs` | validate → signals → filter ops → strategies → filter DD/ROI |
| Which indicators are already done? | `get_unprocessed()` | `src/application/strategy_runner_utils.rs` | Set-diff DB indicator IDs vs manifest |
| One cell's compute result | `CellOutput` | `src/application/cell_output.rs` | instrument, mms_id, strategies, processed IDs |
| Write strategies to parquet | `ParquetStrategyRepository::write_rows()` | `src/infrastructure/strategy_adapter.rs` | Partitioned by instrument + mms_id; ZSTD; idempotent by indicator_id |
| Record processed indicators | `ParquetProcessedManifest` | `src/infrastructure/processed_manifest.rs` | `processed.parquet` ledger; atomic writes; dedup per triple |
| Commit computed cells (create, create_loop) | `StrategyPersistence::persist_cells()` | `src/infrastructure/strategy_persistence.rs` | Assign global IDs, write partitions in parallel, commit manifest once |
| Logging | `logging::init()` | `src/infrastructure/logging.rs` | simplelog, RUST_LOG-driven |
| Engine CLI module | Command tree for `bte` (the sole CLI) | `src/engine/mod.rs` | `EngineCommand` enum + `build_cli()`, `parse()`, `execute()` |
| Engine strategy create | `bte strategy create -m <mms_id> [-i <instrument>]` | `src/engine/strategy/create.rs` | fetch → manifest diff → run_strategies → parquet → manifest append |
| Engine strategy create_loop | `bte strategy create_loop` | `src/engine/strategy/create_loop.rs` | All cells: load once (rayon::join) → parallel compute → batch persist |
| Refresh engine binary after changes | `cargo install --path . --bin bte --force` | `Cargo.toml` defines the single `[[bin]]` entry | Installs updated `bte` to `~/.cargo/bin/bte` |
| Parquet files volume | `${PARQUET_FILES_PATH}:/app/parquet_files` | `docker-compose.yaml`, `.env.example` | Host path configurable via `PARQUET_FILES_PATH` env var |

## Architecture Pattern

```
Business Question                              Code Path
────────────────                                ─────────
"What trades happened?"                   →   Repository trait → get_all() → Vec<OperationPoint>
"Trades for EURUSD + mms 5?"              →   get_by_mms_id_and_instrument(5, "EURUSD")
"Is this data valid?"                     →   combined_validation::validate_all(long, short, indicators, resampled, mms_id)
"What instrument are we on?"              →   any resampled point's instrument (validation requires all to agree)
"What MMS are we testing?"                →   expected_mms_id parameter to validate_all()
"What is the time span of the data?"      →   operation_points.start_end_timestamps() → (start_ts, end_ts)
"How deep was the worst draw down?"       →   performance_metrics(&[&operation_points]).max_draw_down
"What is the total return?"               →   performance_metrics(&[&operation_points]).global_roi
"What is the annualized return?"          →   performance_metrics(&[&operation_points]).annual_roi(start_ts, end_ts)
"How many trades per year?"               →   annual_operation_count(count, start_ts, end_ts)
"Compute indicator values?"               →   create_indicator_output(&parameters, &resampled_points)
"Generate signal groups?"                 →   create_signal_groups(&indicators, &resampled_points)  (real indicator math)
"Which signal groups have enough ops?"    →   filter_signal_groups_by_annual_ops(groups, min_ops, start_ts, end_ts)
"Which strategies pass the quality bar?"  →   filter_strategies(strategies, &GlobalStrategyFilter::default())
"Create strategies for one cell?"         →   bte strategy create -m <mms_id> [-i <instrument>]
"Create strategies for everything?"       →   bte strategy create_loop (load once → parallel cells → batch persist)
"Which indicators are already processed?" →   StrategyPersistence::read_processed_ids_per_cell()
```

## Adding New Features — Where Things Go

| What You're Adding | Where It Goes | Pattern to Follow |
|--------------------|---------------|-------------------|
| New entity | `src/domain/{entity}.rs` + trait | See `indicator.rs` |
| New validation rule | `src/domain/validation/{rule}.rs` | See `same_instrument_validation.rs` |
| New domain metric | `src/domain/{metric}.rs` as a pure fn | See `performance_metrics.rs` (`&[&OperationPoint]` → struct, single pass + defensive sort), `annual_operation_count.rs` (count + span) |
| New indicator | `IndicatorParameters` variant + `src/domain/indicators/{name}.rs` + dispatch arm | See `bears.rs` / `rsi.rs`; use `validate_period!`, `empty_oscillator!`, `indicator_values!` macros |
| New signal semantics | `src/domain/signal/{name}.rs` + `IndicatorOutput` variant | See `oscillator.rs` (zero-cross) |
| New data source adapter | `src/infrastructure/{adapter}.rs` | See `pg_adapter.rs` or `strategy_adapter.rs` |
| New application orchestration | `src/application/{name}.rs` | See `strategy_runner.rs` / `source_data.rs` |
| New CLI command group | `src/engine/{name}/` module dir | Follow `src/engine/strategy/` pattern: `mod.rs` (enum + `build_cli`/`parse`/`execute`) + one file per subcommand |
| New CLI subcommand under existing group | Inside the relevant `src/engine/{group}/` | Add the variant to the enum + builder + parser + execute match |
| New engine command group (bte binary) | `src/engine/{name}/` module dir | Follow `src/engine/strategy/` pattern: mod.rs + one file per subcommand |
| New test fixtures | `src/domain/fixtures.rs` | Add `pub fn {name}() -> Vec<T>` |
| New infrastructure adapter | `src/infrastructure/{name}.rs` | See `processed_manifest.rs` — expose types and functions, add `pub mod` in `mod.rs` |

## Related Files

- `domain-model.md` — Full glossary, entity specs, invariant details
- `business-domain.md` — Why this project exists
- `decisions-log.md` — Architectural decisions
