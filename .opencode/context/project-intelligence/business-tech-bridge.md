<!-- Context: project-intelligence/bridge | Priority: high | Version: 1.3 | Updated: 2026-06-26 -->

# Business ↔ Tech Bridge

> How domain concepts map to code. Use this to navigate from a business question to the right file.

## Core Mapping

| Business Concept | Rust Implementation | File | Notes |
|-----------------|---------------------|------|-------|
| A trade (long or short) | `OperationPoint` struct | `src/domain/operation_point.rs` | Direction determined by repository, not a field |
| Long/short trade bundle | `OperationPoints` struct | `src/domain/operation_point.rs` | Pairs long+short; `*_points_by_timestamp()` lookup maps |
| Long trades collection | `LongOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure |
| Short trades collection | `ShortOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure |
| Daily market candle | `ResampledPointD1` struct | `src/domain/resampled_point.rs` | OHLCV + instrument + timestamp + high_low_order |
| Intraday price ordering | `HighLowOrder` enum | `src/domain/resampled_point.rs` | HighFirst / LowFirst / Undefined |
| Market data collection | `ResampledPointD1Repository` trait | `src/domain/resampled_point.rs` | Port — `get_all()`, `get_by_instrument()` |
| Technical indicator | `Indicator` struct | `src/domain/indicator.rs` | Has `id`, `type`, typed `parameters`, `identifier` |
| Indicator parameters | `IndicatorParameters` enum | `src/domain/indicator.rs` | Macd{...} / Rsi{...} |
| Indicator collection | `IndicatorRepository` trait | `src/domain/indicator.rs` | Port — `get_all()`, `get_by_type()` |
| Money management strategy | `money_management_strategy_id` field | `src/domain/operation_point.rs`, `src/domain/strategy.rs` | Integer ID, no dedicated struct yet |
| A backtested strategy | `Strategy` struct | `src/domain/strategy.rs` | indicator + op-point ID sets + MMS + metrics |
| Strategy quality bar | `GlobalStrategyFilter` + `filter_strategies()` | `src/domain/strategy.rs` | Default: DD≤30%, ROI≥10%, ops≥10 |
| Annual return on investment | `annual_roi` field on `Strategy` | `src/domain/strategy.rs` | Randomized by current generator |
| "How deep did it draw down?" | `max_draw_down()` | `src/domain/max_draw_down.rs` | Pure fn over `&[OperationPoint]`, returns % |
| "How many trades per year?" | `annual_operation_count()` | `src/domain/annual_operation_count.rs` | Pure fn over `&[OperationPoint]` + time span |
| "Is the data valid?" | `validate_all()` | `src/domain/validation/combined_validation.rs` | Runs 5 checks, fails fast |
| "Data must not be empty" | `validate_empty_entities()` | `src/domain/validation/empty_validation.rs` | Checks all 4 datasets |
| "All same instrument" | `validate_same_instrument()` | `src/domain/validation/same_instrument_validation.rs` | Compares against first resampled point |
| "All same indicator type" | `validate_same_indicator_types()` | `src/domain/validation/same_indicator_types_validation.rs` | Compares against first indicator |
| "All same MMS" | `validate_money_management_strategy_id_matches()` | `src/domain/validation/same_mms_validation.rs` | Checks against expected ID param |
| "Timestamps align" | `validate_timestamps_match()` | `src/domain/validation/same_timestamp_validation.rs` | Long ∩ Short ⊆ Resampled |
| Test data | `domain::fixtures::*` functions | `src/domain/fixtures.rs` | Shared across all test modules |
| In-memory storage | `InMemory*Repo` structs | `src/infrastructure/in_memory_repos.rs` | Implement all 4 repository traits |
| Environment config | `Environment` enum + `DbConfig` | `src/config/environment.rs` | Production / Development / Testing |
| Parquet base directory | `parquet_base_dir(env)` | `src/config/environment.rs` | `/app/parquet_files/{env}/strategy` |
| Fetch strategy inputs from DB | `fetch_strategy_data(query)` | `src/application/strategy_data.rs` | `StrategyQuery{mms_id, instrument}` → `StrategyData` |
| Generate strategies from DB data | `generate_random_strategies()` | `src/application/strategy_generator.rs` | One strategy per DB indicator |
| Generate synthetic strategies | `generate_random_strategies_bulk()` | `src/application/strategy_generator.rs` | Distributes across (instrument × mms) partitions; optional seed |
| Write strategies to parquet | `ParquetStrategyRepository::write()` | `src/infrastructure/strategy_adapter.rs` | Partitioned by instrument + mms_id; ZSTD |
| Redis cache (ephemeral) | `SignalGroup`, `SignalGroups`, CRUD functions | `src/infrastructure/redis.rs` | Typed key-value cache for signal groups |
| Redis connection | `redis::Connection` via `redis::Client` | `src/infrastructure/redis.rs` | Sync connection to `redis://redis:6379/0` |
| Redis cache key constant | `SIGNAL_GROUPS_KEY` (`&str`) | `src/infrastructure/redis.rs` | Plain `"signal_groups"` constant |
| Read cached signal groups | `read_signal_groups(conn)` | `src/infrastructure/redis.rs` | Returns `Option<SignalGroups>`, hard fail on error |
| Write cached signal groups | `write_signal_groups(conn, groups)` | `src/infrastructure/redis.rs` | Serializes to JSON, hard fail on error |
| Delete cached signal groups | `delete_signal_groups(conn)` | `src/infrastructure/redis.rs` | DEL key, hard fail on error |
| CLI binary (`ct`) | Full backtesting CLI | `src/main.rs` | `cli::build_cli()` → `cli::parse()` → `cli::execute()`. Installed at `~/.cargo/bin/ct` |
| CLI dev tools | `ct dev create_indicators`, `ct dev create_strategies` | `src/cli/dev.rs` | Each file has its own builder, domain types, parser, and execute |
| CLI indicator | `ct indicator/i create macd/rsi`, `bt i delete`, `bt i delete_unused` | `src/cli/indicator.rs` | Nested `create` subcommand with `macd` and `rsi` leaves |
| CLI MMS | `ct money-management-strategy/mms create`, `ct mms delete` | `src/cli/money_management_strategy.rs` | |
| CLI operation point | `ct operation-point/o align`, `ct o create`, `ct o delete` | `src/cli/operation_point.rs` | |
| CLI raw point | `ct raw-point/r create`, `ct r delete` | `src/cli/raw_point.rs` | |
| CLI redis cache | `ct redis` (check), `ct redis -v` (verbose), `ct redis --clean` | `src/cli/redis.rs` | Delegates to `bt-engine redis` via Docker exec |
| CLI resampled point | `ct resampled-point/rr create`, `ct rr delete` | `src/cli/resampled_point.rs` | |
| CLI strategy create | `ct strategy/s create -m <mms_id> [-i <instrument>]` | `src/cli/strategy.rs` | Forwards to `bt-engine strategy create` via Docker exec |
| CLI strategy generate | `ct s generate -c <n> [-i ...] [-m <mms>] [-p <ops>] [-s <seed>] [--parquet-dir <dir>]` | `src/cli/strategy.rs` | Forwards to `bt-engine strategy generate` |
| CLI strategy display | `ct s display <strategy_id> [-v/--verbose]` | `src/cli/strategy.rs` | Forwards to Python `bt strategy display` via Docker exec |
| Engine CLI module | Command tree for `bt-engine` | `src/engine/mod.rs` | `EngineCommand` enum + `build_cli()`, `parse()`, `execute()` |
| Engine strategy create | `bt-engine strategy create -m <mms_id> [-i <instrument>]` | `src/engine/strategy.rs` | Fetches from PG → validates → generates → writes parquet |
| Engine strategy generate | `bt-engine strategy generate ...` | `src/engine/strategy.rs` | Purely synthetic; no DB; writes partitioned parquet |
| Engine redis subcommand | Redis cache under `bt-engine` | `src/engine/redis.rs` | `RedisCommand::Check { verbose }`, `RedisCommand::Clean` |
| Refresh CLI after changes | `cargo install --path . --bin ct --force` | — | Installs updated `ct` to `~/.cargo/bin/ct` |
| Refresh engine binary after changes | `cargo install --path . --bin bt-engine --force` | `Cargo.toml` defines both `[[bin]]` entries | Installs updated `bt-engine` to `~/.cargo/bin/bt-engine` |
| Docker Redis service | `redis:7-alpine` container | `docker-compose.yaml` | Named volume `redis-data`, port 6379, container_name: redis |
| Parquet files volume | `${PARQUET_FILES_PATH}:/app/parquet_files` | `docker-compose.yaml`, `.env.example` | Host path configurable via `PARQUET_FILES_PATH` env var |

## Architecture Pattern

```
Business Question                              Code Path
────────────────                              ─────────
"What trades happened?"                   →   Repository trait → get_all() → Vec<OperationPoint>
"Trades for EURUSD + mms 5?"              →   get_by_mms_id_and_instrument(5, "EURUSD")
"Is this data valid?"                     →   combined_validation::validate_all(long, short, indicators, resampled, mms_id)
"What instrument are we on?"              →   resampled_points[0].instrument (reference instrument)
"What MMS are we testing?"                →   expected_mms_id parameter to validate_all()
"How deep was the worst draw down?"       →   max_draw_down(&operation_points)
"How many trades per year?"               →   annual_operation_count(&ops, start_ts, end_ts)
"Which strategies pass the quality bar?"  →   filter_strategies(strategies, &GlobalStrategyFilter::default())
"Generate strategies from DB data?"       →   fetch_strategy_data() → generate_random_strategies() → ParquetStrategyRepository::write()
"Generate N synthetic strategies?"        →   generate_random_strategies_bulk(count, instruments, mms_count, ...) → write per partition
"Is the cache HIT or MISS?"               →   bt-engine redis / ct redis → read_signal_groups()
"Clear the signal cache?"                 →   bt-engine redis --clean / ct redis --clean → delete_signal_groups()
```

## Adding New Features — Where Things Go

| What You're Adding | Where It Goes | Pattern to Follow |
|--------------------|---------------|-------------------|
| New entity | `src/domain/{entity}.rs` + trait | See `indicator.rs` |
| New validation rule | `src/domain/validation/{rule}.rs` | See `same_instrument_validation.rs` |
| New domain metric | `src/domain/{metric}.rs` as a free fn over `&[OperationPoint]` | See `max_draw_down.rs` / `annual_operation_count.rs` |
| New data source adapter | `src/infrastructure/{adapter}.rs` | See `in_memory_repos.rs` or `redis.rs` |
| New application orchestration | `src/application/{name}.rs` | See `strategy_data.rs` / `strategy_generator.rs` |
| New CLI command group (ct binary) | `src/cli/{name}.rs` | See `src/cli/dev.rs` — 4 sections: builder, types, parse, execute |
| New CLI subcommand under existing group | Inside the relevant `src/cli/{group}.rs` or `src/engine/{group}.rs` | Add to the enum + builder + parser + execute match |
| New engine command group (bt-engine binary) | `src/engine/{name}.rs` | Follow `src/engine/strategy.rs` pattern: builder, types, parse, execute |
| New engine subcommand under existing group | Inside the relevant `src/engine/{group}.rs` | Add variant to EngineCommand enum + match arms in build_cli() and parse() |
| New test fixtures | `src/domain/fixtures.rs` | Add `pub fn {name}() -> Vec<T>` |
| New infrastructure adapter | `src/infrastructure/{name}.rs` | See `strategy_adapter.rs` — expose types and functions, add `pub mod` in `mod.rs` |

## Related Files

- `domain-model.md` — Full glossary, entity specs, invariant details
- `business-domain.md` — Why this project exists
- `decisions-log.md` — Architectural decisions
