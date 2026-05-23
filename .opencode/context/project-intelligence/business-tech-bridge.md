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
| New API endpoint | `src/bin/server.rs` → `configure_routes` | See `health_check` handler |
| New test fixtures | `src/domain/fixtures.rs` | Add `pub fn {name}() -> Vec<T>` |

## Related Files

- `domain-model.md` — Full glossary, entity specs, invariant details
- `business-domain.md` — Why this project exists
- `decisions-log.md` — Architectural decisions
