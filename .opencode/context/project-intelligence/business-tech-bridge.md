<!-- Context: project-intelligence/bridge | Priority: high | Version: 1.0 | Updated: 2026-05-23 -->

# Business ↔ Tech Bridge

> How domain concepts map to code. Use this to navigate from a business question to the right file.

## Core Mapping

| Business Concept | Rust Implementation | File | Notes |
|-----------------|---------------------|------|-------|
| A trade (long or short) | `OperationPoint` struct | `src/domain/operation_point.rs` | Direction determined by repository, not a field |
| Long trades collection | `LongOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure |
| Short trades collection | `ShortOperationPointRepository` trait | `src/domain/operation_point.rs` | Port — impl in infrastructure |
| Daily market candle | `ResampledPointD1` struct | `src/domain/resampled_point.rs` | OHLCV + instrument + timestamp |
| Market data collection | `ResampledPointD1Repository` trait | `src/domain/resampled_point.rs` | Port — impl in infrastructure |
| Technical indicator | `Indicator` struct | `src/domain/indicator.rs` | Has `id` and `type` |
| Indicator collection | `IndicatorRepository` trait | `src/domain/indicator.rs` | Port — impl in infrastructure |
| Money management strategy | `money_management_strategy_id` field on `OperationPoint` | `src/domain/operation_point.rs` | Integer ID, no dedicated struct yet |
| "Is the data valid?" | `validate_all()` | `src/domain/validation/combined_validation.rs` | Runs 5 checks, fails fast |
| "Data must not be empty" | `validate_empty_entities()` | `src/domain/validation/empty_validation.rs` | Checks all 4 datasets |
| "All same instrument" | `validate_same_instrument()` | `src/domain/validation/same_instrument_validation.rs` | Compares against first resampled point |
| "All same indicator type" | `validate_same_indicator_types()` | `src/domain/validation/same_indicator_types_validation.rs` | Compares against first indicator |
| "All same MMS" | `validate_money_management_strategy_id_matches()` | `src/domain/validation/same_mms_validation.rs` | Checks against expected ID param |
| "Timestamps align" | `validate_timestamps_match()` | `src/domain/validation/same_timestamp_validation.rs` | Long ∩ Short ⊆ Resampled |
| Test data | `domain::fixtures::*` functions | `src/domain/fixtures.rs` | Shared across all test modules |
| In-memory storage | `InMemory*Repo` structs | `src/infrastructure/in_memory_repos.rs` | Implement all 4 repository traits |
| Environment config | `Environment` enum + `DbConfig` | `src/config/environment.rs` | Production / Development / Testing |
| CLI binary | `bt` (installed at `~/.cargo/bin/bt`) | `src/main.rs` | Entry point: `build_cli()` → `parse()` → `execute()` |
| CLI dev tools | `bt dev create_indicators`, `bt dev create_strategies` | `src/cli/dev.rs` | Each file has its own builder, domain types, parser, and execute |
| CLI indicator | `bt indicator/i create macd/rsi`, `bt i delete`, `bt i delete_unused` | `src/cli/indicator.rs` | Nested `create` subcommand with `macd` and `rsi` leaves |
| CLI MMS | `bt money-management-strategy/mms create`, `bt mms delete` | `src/cli/money_management_strategy.rs` | |
| CLI operation point | `bt operation-point/o align`, `bt o create`, `bt o delete` | `src/cli/operation_point.rs` | |
| CLI raw point | `bt raw-point/r create`, `bt r delete` | `src/cli/raw_point.rs` | |
| CLI redis | `bt redis get`, `bt redis delete` | `src/cli/redis.rs` | |
| CLI resampled point | `bt resampled-point/rr create`, `bt rr delete` | `src/cli/resampled_point.rs` | |
| CLI strategy | `bt strategy/s create/create_one`, `bt s delete/display_table/terminate` | `src/cli/strategy.rs` | Nested `filters` subcommand with `custom` and `global` leaves |
| CLI strategy filters | `bt s filters custom/global` | `src/cli/strategy.rs` | Internal `build_strategy_filters_cli()`, `parse_strategy_filters()` |
| Refresh CLI after changes | `bt-refresh` (zsh alias) | runs `cargo install --path . --bin bt --force` | Installs updated `bt` to `~/.cargo/bin/bt` |
| HTTP server | Actix-web app | `src/bin/server.rs` | Health check at `/health` only |

## Architecture Pattern

```
Business Question                    Code Path
─────────────────                    ─────────
"What trades happened?"         →    Repository trait → get_all() → Vec<OperationPoint>
"Is this data valid?"           →    combined_validation::validate_all(long, short, indicators, resampled, mms_id)
"What instrument are we on?"    →    resampled_points[0].instrument (reference instrument)
"What MMS are we testing?"      →    expected_mms_id parameter to validate_all()
```

## Adding New Features — Where Things Go

| What You're Adding | Where It Goes | Pattern to Follow |
|--------------------|---------------|-------------------|
| New entity | `src/domain/{entity}.rs` + trait | See `indicator.rs` |
| New validation rule | `src/domain/validation/{rule}.rs` | See `same_instrument_validation.rs` |
| New data source adapter | `src/infrastructure/{adapter}.rs` | See `in_memory_repos.rs` |
| New CLI command group | `src/cli/{name}.rs` | See `src/cli/dev.rs` — 4 sections: builder, types, parse, execute |
| New CLI subcommand under existing group | Inside the relevant `src/cli/{group}.rs` | Add to the enum + builder + parser + execute match |
| New API endpoint | `src/bin/server.rs` → `configure_routes` | See `health_check` handler |
| New test fixtures | `src/domain/fixtures.rs` | Add `pub fn {name}() -> Vec<T>` |

## Related Files

- `domain-model.md` — Full glossary, entity specs, invariant details
- `business-domain.md` — Why this project exists
- `decisions-log.md` — Architectural decisions
