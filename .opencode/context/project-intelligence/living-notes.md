<!-- Context: project-intelligence/notes | Priority: high | Version: 1.5 | Updated: 2026-08-19 -->

# Living Notes

> Active issues, technical debt, open questions, and insights that don't fit elsewhere. Keep this alive.

## Quick Reference

- **Purpose**: Capture current state, problems, and open questions
- **Update**: Weekly or when status changes
- **Archive**: Move resolved items to bottom with status

## Technical Debt

| Item | Impact | Priority | Mitigation |
|------|--------|----------|------------|
| Server scaffold removed (no API layer) | HTTP endpoints must be built from scratch if needed | Low | Not needed until API consumers exist |
| Engine module has no unit tests | Regression risk for engine CLI commands | Medium | Add tests for StrategyCommand::execute() |
| Signal generator uses random timestamps | `create_signals()` picks random timestamps from resampled points — the signals themselves are placeholders, not derived from indicator logic | High | Implement real signal generation from indicator parameters (MACd crossover, RSI threshold crossings, etc.) |
| `ct strategy display` forwards to Python CLI | Display logic lives in Backtesting-4.0, not the Rust engine | Medium | Port display logic to Rust engine if/when Python dependency is dropped |
| PG adapter hand-parses indicator JSON | `json_extract_str`/`json_extract_i32` are panic-on-miss string scanners, not a real JSON parser | Medium | Replace with `serde_json` parsing once `serde` is on the hot path, or keep for performance |

### Technical Debt Details

**Missing HTTP API layer**
*Priority*: Low
*Impact*: No REST/HTTP interface exists for the backtesting engine. CLI-only.
*Root Cause*: Actix-web scaffold was removed as dead code in commit 1f646c9. Deliberate -- no API consumers yet.
*Proposed Solution*: Add API layer when a use case emerges (web UI, external service integration).
*Effort*: Large
*Status*: Deferred

**Engine module has no unit tests**
*Priority*: Medium
*Impact*: `bt-engine` CLI commands (strategy create) are untested at the integration level. Regression risk when adding new engine commands.
*Root Cause*: Engine module is newly extracted. Tests not yet written.
*Proposed Solution*: Add integration tests that exercise engine commands against a real PG instance (in CI) or use mocks.
*Effort*: Medium
*Status*: Known

**Random placeholder signal timestamps**
*Priority*: High
*Impact*: `create_signals()` in `application::signal_generator` generates `SignalGroup`s by picking random timestamps from resampled points (skipping first/last 200 candles). The signals are not derived from actual indicator logic (MACD crossover, RSI threshold crossings, etc.). While the metrics computed FROM these signals are real, the signals themselves are placeholders.
*Root Cause*: Real signal generation from indicator parameters is not yet implemented.
*Proposed Solution*: Implement indicator-specific signal generation: MACD crossover signals (fast/slow line intersections), RSI threshold crossing signals (high/low threshold breaches), etc.
*Effort*: Large
*Status*: Known — next iteration

**~~Random placeholder strategy metrics~~ (RESOLVED 2026-06-28)**
*Priority*: ~~High~~ Resolved
*Impact*: ~~`generate_random_strategies` and `generate_random_strategies_bulk` fill `annual_roi`, `max_draw_down`, and `annual_operation_count` with random values~~
*Root Cause*: ~~The pure metric functions existed but were not wired into the generator.~~
*Resolution*: `create_strategies()` now computes real `max_draw_down`, `global_roi` → `annual_roi`, and `annual_operation_count` from each strategy's referenced operation points. Parallelized with rayon. See decisions-log.md "Compute Real Metrics in the Strategy Generator".

## Open Questions

| Question | Stakeholders | Status | Next Action |
|----------|--------------|--------|-------------|
| Should engine commands be extended with strategy delete/update? | Arman Mojaver | Open | Add commands as needed |
| Should the ct binary forward engine commands to bt-engine? | Arman Mojaver | Open | Currently separate binaries |
| Should `ct strategy display` be ported to the Rust engine? | Arman Mojaver | Open | Currently forwards to Python `bt strategy display` |
| Should signal generation be indicator-specific (MACD/RSI logic) or remain random? | Arman Mojaver | Open | Implement real signal generation from indicator parameters |

### Open Question Details

**Engine command scope**
*Context*: Currently `bt-engine strategy` has `create` and `generate`. The `ct strategy` CLI group (Python) has more commands (delete, display_table, terminate, filters). Should these be duplicated in the Rust engine?
*Stakeholders*: Arman Mojaver
*Options*:
- Option A: Add all strategy commands to `bt-engine` (Rust-native)
- Option B: Keep `ct` as the frontend that dispatches to either Python `bt` CLI or `bt-engine`
- Option C: Keep them separate
*Timeline*: No deadline
*Status*: Open

**Strategy display ownership**
*Context*: `ct strategy display <id> [-v]` forwards to the Python CLI (`bt strategy display`) via Docker exec. The Rust engine has no display command. If the Python dependency is dropped, display logic must be ported.
*Stakeholders*: Arman Mojaver
*Options*:
- Option A: Port display (iteration table rendering) to the Rust engine
- Option B: Keep display in Python indefinitely
*Timeline*: No deadline
*Status*: Open

## Known Issues

| Issue | Severity | Workaround | Status |
|-------|----------|------------|--------|
| No integration tests for engine module | Medium | Manual verification | Known |
| Signal timestamps are random, not indicator-derived | High | None yet — signals are placeholders | Known |

### Issue Details

**Engine module has no integration tests**
*Severity*: Medium
*Impact*: `bt-engine` CLI commands (strategy create) are untested at the integration level.
*Reproduction*: N/A -- missing test coverage.
*Root Cause*: Engine module is newly extracted. Tests not yet written.
*Fix Plan*: Add integration tests that spin up a PG instance (or use a mock) and exercise the engine CLI flow.
*Status*: Known

**Signal timestamps are random, not indicator-derived**
*Severity*: High
*Impact*: `SignalGroup`s are generated by picking random timestamps from resampled points. The signals do not reflect actual indicator logic (MACD crossover, RSI threshold crossings). Metrics computed from these signals are real but the signals themselves are placeholders.
*Root Cause*: Real signal generation from indicator parameters is not yet implemented.
*Fix Plan*: Implement indicator-specific signal generation logic.
*Status*: Known — next iteration

**~~Generated strategy metrics are random~~ (RESOLVED 2026-06-28)**
*Severity*: ~~High~~ Resolved
*Resolution*: `create_strategies()` now computes real metrics from operation points. See decisions-log.md.

## Insights & Lessons Learned

### What Works Well
- **`build -> parse -> execute` pattern** -- Consistent across both binaries. Simple, predictable, easy to test.
- **Pure validators as free functions** -- No object construction needed. Compose well in `combined_validation::validate_all()`.
- **Pure metric functions** -- `max_draw_down()` and `annual_operation_count()` follow the same free-function-over-slices pattern as validators. Independently unit-tested, no I/O.
- **Separate engine binary** -- Clear line between Python-wrapping CLI (`ct`) and native Rust engine (`bt-engine`).
- **Application layer** -- `fetch_strategy_data()` + `create_signals()` + `create_strategies()` keep `engine/` CLI handlers thin and reusable across binaries.
- **Parquet partitioning by (instrument, mms_id)** -- Mirrors the single-instrument/single-MMS backtest constraint; enables partition pruning.
- **Real metrics in generator** -- `create_strategies()` computes real `max_draw_down`/`global_roi`/`annual_roi`/`annual_operation_count` from operation points. Parallelized with rayon.

### What Could Be Better
- **Test coverage** -- No integration tests for the engine module yet. Should have tests for `StrategyCommand::execute()`.
- **Real signal generation** -- `create_signals()` picks random timestamps. Real indicator-based signal generation (MACD crossover, RSI thresholds) is the next step.

### Lessons Learned
- **Keep context files in sync with code** -- After major refactors, update `.opencode/context/` immediately. Outdated context misleads AI agents.
- **Delete dead code early** -- Removing the server scaffold saved ~1700 lines from Cargo.lock and eliminated misleading references to Actix-web dependencies.
- **One file per metric** -- `max_draw_down.rs` and `annual_operation_count.rs` are each independently readable and testable. Avoids a monolithic `metrics.rs`.

## Patterns & Conventions

### Code Patterns Worth Preserving
- **`build -> parse -> execute`** -- Every binary follows this 3-step pattern. Lives in `cli/mod.rs` and `engine/mod.rs`. Consistent, predictable.
- **Pure validation functions** -- `src/domain/validation/*.rs`. Each is a free function taking slices, returning `Result`. Composable.
- **Pure metric functions** -- `src/domain/max_draw_down.rs`, `src/domain/annual_operation_count.rs`. Free functions over `&[OperationPoint]`. Composable, unit-tested in-file.
- **`GlobalStrategyFilter::default()`** -- Encodes the baseline quality bar in code (DD≤30%, ROI≥10%, ops≥10). Override per-call, default for convenience.
- **Fixture module** -- `domain::fixtures.rs` provides reusable test data across all test modules. Single source of truth.
- **Application orchestration** -- `src/application/{name}.rs` — one file per use case. Keeps `engine/` thin.
- **Parquet partition layout** -- `instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. Matches backtest constraints.

### Gotchas for Maintainers
- **Two binaries to install** -- After changes to `engine/`, run `cargo install --path . --bin bt-engine --force`. After changes to `cli/`, run `cargo install --path . --bin ct --force`. Or use `--all` to install both.
- **Python CLI dependency** -- `ct` commands (dev, indicator, display, etc.) forward to a Python Docker container. The `bt-engine` binary does NOT depend on Docker.
- **Parquet base dir is environment-scoped** -- `/app/parquet_files/{env}/strategy`. Override with `--parquet-dir` when running outside Docker.
- **Signal timestamps are placeholders** -- `create_signals()` picks random timestamps, not indicator-derived signals. Metrics are real given the signals, but the signals themselves are random.

## Active Projects

| Project | Goal | Owner | Timeline |
|---------|------|-------|----------|
| Engine CLI extraction | Move Rust engine commands into standalone `bt-engine` binary | Arman Mojaver | Complete (2026-06-04) |
| Strategy generation + parquet persistence | Generate `Strategy` records and write partitioned parquet files | Arman Mojaver | Complete (2026-06-08) |
| Application layer | Orchestrate fetch → validate → generate → persist in `src/application/` | Arman Mojaver | Complete (2026-06-07) |
| Domain metric functions | `max_draw_down()` + `annual_operation_count()` as pure domain functions | Arman Mojaver | Complete (2026-06-24) |
| Strategy filtering | `filter_strategies()` + `GlobalStrategyFilter` with baseline defaults | Arman Mojaver | Complete (2026-06-22) |
| Wire real metrics into generator | Replace random placeholder metrics with computed `max_draw_down`/`global_roi`/`annual_roi`/`annual_operation_count` | Arman Mojaver | Complete (2026-06-28) |

## Archive (Resolved Items)

Moved here for historical reference. Current team should refer to current notes above.

### Resolved: Server scaffold removal
- **Resolved**: 2026-06-04
- **Resolution**: Actix-web server scaffold (`src/bin/server.rs`) removed. Dependencies cleaned up from Cargo.toml.
- **Learnings**: Don't add API layer until there are consumers. CLI-only is sufficient for now.

### Resolved: Engine module extraction
- **Resolved**: 2026-06-04
- **Resolution**: Engine commands extracted from `cli/mod.rs` into `src/engine/` module with own binary (`bt-engine`).
- **Learnings**: Clear separation between Python-wrapping CLI and native Rust engine reduces confusion and Docker indirection.

### Resolved: Strategy generation + parquet persistence
- **Resolved**: 2026-06-08
- **Resolution**: `Strategy` entity, `strategy_generator` application module, `ParquetStrategyRepository`, and `strategy create`/`generate` CLI commands added. Pipeline now persists output to partitioned parquet files.
- **Learnings**: Partitioning by (instrument, mms_id) mirrors backtest constraints and enables partition pruning. A DB-free generate mode enables reproducible test fixtures.

### Resolved: Application layer introduction
- **Resolved**: 2026-06-07
- **Resolution**: `src/application/` module added with `strategy_data.rs` (fetch) and `strategy_generator.rs` (generate). `engine/` CLI handlers kept thin.
- **Learnings**: A dedicated application layer keeps CLI handlers thin and use-case logic reusable across binaries.

### Resolved: Domain metric functions
- **Resolved**: 2026-06-24
- **Resolution**: `max_draw_down()` and `annual_operation_count()` added as pure free functions over `&[OperationPoint]` in `src/domain/`. `filter_strategies()` + `GlobalStrategyFilter` added to `src/domain/strategy.rs`.
- **Learnings**: One file per metric keeps each independently readable and testable. `GlobalStrategyFilter::default()` documents the baseline quality bar in code.

### Resolved: Real metrics in the strategy generator
- **Resolved**: 2026-06-28
- **Resolution**: `create_strategies()` now computes real `max_draw_down`, `global_roi` → `annual_roi`, and `annual_operation_count` from each strategy's referenced operation points. Added `global_roi()` and `annual_roi()` domain functions. Parallelized with rayon. `strategy generate` command removed (no longer useful with real metrics).
- **Learnings**: Two-stage filtering (signal groups by op count, then strategies by DD + ROI) reduces wasted computation. Rayon `par_iter()` is trivial to add for embarrassingly parallel work.

### Resolved: i64 timestamps + SignalGroup in domain
- **Resolved**: 2026-06-28
- **Resolution**: All timestamps converted to `i64`. `SignalGroup` moved to `domain::signal_group` with embedded `indicator_id`. The cache now stores `Vec<SignalGroup>`.
- **Learnings**: Domain concepts belong in the domain layer, even if they're cached in infrastructure. Embedding the ID in the struct survives serialization better than a HashMap key.

## Onboarding Checklist

- [ ] Review known technical debt and understand impact
- [ ] Know what open questions exist and who's involved
- [ ] Understand current issues and workarounds
- [ ] Be aware of patterns and gotchas
- [ ] Know active projects and timelines
- [ ] Understand the team's priorities

## Related Files

- `decisions-log.md` - Past decisions that inform current state
- `business-domain.md` - Business context for current priorities
- `business-tech-bridge.md` - Context for current trade-offs
