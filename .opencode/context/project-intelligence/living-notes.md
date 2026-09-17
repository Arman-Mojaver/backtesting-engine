<!-- Context: project-intelligence/notes | Priority: high | Version: 1.7 | Updated: 2026-09-03 -->

# Living Notes

> Active issues, technical debt, open questions, and insights that don't fit elsewhere. Keep this alive.

## Quick Reference

- **Purpose**: Capture current state, problems, and open questions
- **Update**: Weekly or when status changes
- **Archive**: Move resolved items to bottom with status

## Technical Debt

| Item | Impact | Priority | Mitigation |
|------|--------|----------|------------|
| Metrics assume operations close in entry order | ROI/max_draw_down walk operation points sorted by entry timestamp; real trades have exit dates and do not necessarily close in entry order — figures are approximate until fixed | **High** | Account for `exit_date` in the compounding curve before any production use (see `notes.md`) |
| MACD indicator unsupported | MACD rows parse from DB but `create_indicator_output` returns `UnsupportedIndicator`; MACD strategies are silently skipped | Medium | Implement MACD compute (fast/slow MA diff + signal line → Crossover output kind) |
| `ct strategy display` forwards to Python CLI | Display logic lives in Backtesting-4.0, not the Rust engine | Medium | Port display logic to Rust engine if/when Python dependency is dropped |
| PG adapter hand-parses indicator JSON | `json_extract_str`/`json_extract_i32` are panic-on-miss string scanners, not a real JSON parser | Medium | Replace with `serde_json` parsing once `serde` is on the hot path, or keep for performance |
| DbConfig is hardcoded per environment | Host (`host.docker.internal`), ports, and credentials cannot be overridden by env vars | Medium | Fine for the Docker workflow; add env overrides if the engine must target arbitrary PG instances |
| Engine CLI handlers have no direct unit tests | `create.rs` / `create_loop.rs` / `metadata.rs` handlers are thin but untested; the functions they call ARE tested | Low-Medium | Logic is pushed into tested application/infrastructure functions; add handler tests if handlers grow |
| Parquet write cost at scale | Partition append = full file rewrite; `max_strategy_id()` scans all partitions | Low | Fine at current scale; rethink if partition counts grow by orders of magnitude |

### Technical Debt Details

**Exit-date ordering in metrics**
*Priority*: High
*Impact*: `performance_metrics()` sorts operation points by entry timestamp and compounds in that order. Real trades close at `exit_date`, which is not necessarily in entry order. ROI and max draw down are approximations until this is modeled.
*Root Cause*: The domain `OperationPoint` has no `exit_date` field; the compounding curve was built on entry timestamps.
*Proposed Solution*: Add `exit_date` to `OperationPoint` (DB + domain + adapter), sort/compound on exit order.
*Effort*: Medium
*Status*: Known — required before production use (also recorded in `notes.md`)

**MACD indicator unsupported**
*Priority*: Medium
*Impact*: `IndicatorParameters::Macd` parses from the database, but `create_indicator_output` returns `IndicatorError::UnsupportedIndicator`. The signal generator skips MACD indicators (`filter_map`), so runs continue without them — but MACD strategies are never produced.
*Root Cause*: MACD needs a two-series (fast/slow + signal) computation that doesn't fit the single-series oscillator pattern yet; the `Crossover` output kind exists and is the natural target.
*Proposed Solution*: Implement MACD compute returning `IndicatorOutput::Crossover`.
*Effort*: Medium
*Status*: Known

**~~Random placeholder signal timestamps~~ (RESOLVED)**
*Priority*: ~~High~~ Resolved
*Resolution*: `create_signal_groups()` now computes real indicator outputs via `create_indicator_output()` (14 indicators) and derives signals via zero-cross / threshold / crossover semantics in `domain/signal/`. MACD remains the one unsupported indicator (skipped, not aborted).

**~~Engine module has no unit tests~~ (PARTIALLY RESOLVED)**
*Resolution*: The logic the engine handlers call is now tested: `application` (signal_generator 5, metadata_report 11, strategy_runner_utils 5), `infrastructure` (strategy_adapter 13, processed_manifest 8, strategy_persistence 1, logging 5). The thin `engine/strategy/*.rs` handlers themselves remain untested — acceptable while they stay thin.

**~~Missing HTTP API layer~~ (DEFERRED — deliberate)**
*Status*: Deferred (YAGNI). Actix-web scaffold removed 2026-06-04. Re-add when API consumers exist.

## Open Questions

| Question | Stakeholders | Status | Next Action |
|----------|--------------|--------|-------------|
| Should engine commands be extended with strategy delete/update? | Arman Mojaver | Open | Add commands as needed |
| Should `ct strategy display` be ported to the Rust engine? | Arman Mojaver | Open | Currently forwards to Python `bt strategy display` |
| When should exit-date-aware metrics land? | Arman Mojaver | Open | Required before production use of ROI/DD figures |
| Should MACD land as Crossover output kind? | Arman Mojaver | Open | Natural fit; needs two-series compute |

## Known Issues

| Issue | Severity | Workaround | Status |
|-------|----------|------------|--------|
| ROI/DD approximate due to entry-order compounding | High | None — model limitation | Known |
| MACD indicators skipped in strategy generation | Medium | Runs continue without MACD strategies | Known |

## Insights & Lessons Learned

### What Works Well
- **`build -> parse -> execute` pattern** — Consistent across both binaries. Simple, predictable, easy to test.
- **Pure validators as free functions** — No object construction needed. Compose well in `combined_validation::validate_all()`.
- **Pure metric functions** — `performance_metrics()` (single pass: global ROI + max draw down, owns the defensive chronological sort) and `annual_operation_count()`. Independently unit-tested, no I/O.
- **Typed indicator parameters + dispatch** — Adding an indicator is mechanical: variant + compute fn + dispatch arm + tests. The `validate_period!` / `empty_oscillator!` / `indicator_values!` macros keep the 14 implementations uniform.
- **Signal semantics as a separate concept** — `IndicatorOutput` (what the indicator computed) is deliberately separate from `SignalGroup` (when to trade). Oscillator/threshold/crossover semantics cover all 14 indicators with three small modules.
- **Separate engine binary** — Clear line between Python-wrapping CLI (`ct`) and native Rust engine (`bt-engine`).
- **Application layer** — `run_strategies()` keeps `engine/` CLI handlers thin and reusable across binaries.
- **Parquet partitioning by (instrument, mms_id)** — Mirrors the single-instrument/single-MMS backtest constraint; enables partition pruning.
- **Processed manifest** — Idempotency ledger makes `create`/`create_loop` safely re-runnable; `metadata` gives operators a progress report for free.
- **Batch parallelism in create_loop** — Load each table once (`rayon::join`), index in memory, compute cells with `par_iter`, persist in one batch. Scales without changing the per-cell logic.

### What Could Be Better
- **Exit-date modeling** — The biggest correctness gap (see Technical Debt).
- **MACD coverage** — The one indicator that parses but doesn't compute.

### Lessons Learned
- **Keep context files in sync with code** — After major refactors, update `.opencode/context/` immediately. Outdated context misleads AI agents. (This file was refreshed 2026-09-03 after the indicators/signal/manifest work landed.)
- **Delete dead code early** — Removing the server scaffold and later the in-memory repos saved maintenance and eliminated misleading references.
- **One file per metric, except coupled metrics** — `annual_operation_count.rs` stands alone; `global_roi`/`max_draw_down`/`annual_roi` live together in `performance_metrics.rs` because they share one compounding curve (single pass).
- **Idempotency belongs at the boundary** — Dedup by `indicator_id` at the parquet partition AND a manifest ledger at the command level: neither alone was sufficient.

## Patterns & Conventions

### Code Patterns Worth Preserving
- **`build -> parse -> execute`** — Every binary follows this 3-step pattern. Lives in `cli/mod.rs` and `engine/mod.rs`.
- **Pure validation functions** — `src/domain/validation/*.rs`. Each is a free function taking slices, returning `Result`. Composable.
- **Pure metric functions** — `src/domain/performance_metrics.rs` (single-pass `PerformanceMetrics` + `annual_roi` method), `src/domain/annual_operation_count.rs`.
- **Indicator macros** — `validate_period!`, `empty_oscillator!`, `indicator_values!` in `src/domain/indicators/macros.rs`.
- **`GlobalStrategyFilter::default()`** — Encodes the baseline quality bar in code (DD≤30%, ROI≥5%, ops≥10/yr). Override per-call, default for convenience.
- **Fixture module** — `domain::fixtures.rs` provides reusable test data across all test modules. Single source of truth.
- **Application orchestration** — `src/application/{name}.rs` — one file per use case. Keeps `engine/` thin.
- **Parquet partition layout** — `instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. Matches backtest constraints.
- **Atomic parquet writes** — temp file + rename (manifest); read-dedup-rewrite (partitions).

- **README stability policy** — The README contains only stable information: abstract domain concepts, CLI surface, data contracts (parquet schema), formulas, tools, and external dependencies. NO function/struct/file names, no folder trees, no test counts — anything that moves with the code lives in these context files, not the README. Rule of thumb: if a code change would force a README edit, that content belongs in `.opencode/context/` instead.

### Gotchas for Maintainers
- **Two binaries to install** — After changes to `engine/`, run `cargo install --path . --bin bt-engine --force`. After changes to `cli/`, run `cargo install --path . --bin ct --force`.
- **Python CLI dependency** — `ct` data-management commands and `strategy display` forward to a Python Docker container (`api`). Engine commands forward to the `engine` container. `bt-engine` itself has no Python dependency.
- **Parquet dirs are environment-scoped** — `/app/parquet_files/{env}/strategy` (partitions) and `/app/parquet_files/{env}/processed.parquet` (manifest). Host-mounted via `PARQUET_FILES_PATH`.
- **DB config is hardcoded per environment** — `host.docker.internal` with ports 5432/54320/54321 for production/development/testing. No env override.
- **MACD is skipped, not an error** — Runs containing MACD indicators silently produce no MACD strategies.
- **Metrics are entry-order approximations** — See Technical Debt (exit dates) before quoting ROI/DD numbers.

## Active Projects

| Project | Goal | Owner | Timeline |
|---------|------|-------|----------|
| Indicator computation module | 14 indicators as pure domain functions + dispatch + macros | Arman Mojaver | Complete |
| Real signal generation | Indicator outputs → SignalGroup via oscillator/threshold/crossover semantics | Arman Mojaver | Complete |
| Processed manifest + idempotency | Ledger of (instrument, mms_id, indicator_id) triples; atomic writes | Arman Mojaver | Complete |
| create_loop batch pipeline | All cells: load once, parallel compute, batch persist | Arman Mojaver | Complete |
| metadata command | DB vs manifest progress report | Arman Mojaver | Complete |
| CI workflow | fmt + clippy -D warnings + tests + Docker build/push (pinned 1.98.1) | Arman Mojaver | Complete |
| Exit-date-aware metrics | Compound on exit order, not entry order | Arman Mojaver | Pending — high priority |
| MACD compute | Crossover-kind output for MACD parameters | Arman Mojaver | Pending |

## Archive (Resolved Items)

Moved here for historical reference. Current team should refer to current notes above.

### Resolved: Real signal generation (2026, prior to 2026-09-03 audit)
- **Resolution**: `create_signal_groups()` computes real indicator outputs (`create_indicator_output`, 14 indicators) and derives signals via `domain/signal/` semantics (oscillator zero-cross, threshold crossing, crossover diff). Replaces the earlier random-timestamp placeholder implementation. MACD remains unsupported (skipped).
- **Learnings**: Separating "indicator output" from "signal semantics" let three small signal modules cover every indicator kind.

### Resolved: Server scaffold removal (2026-06-04)
- **Resolution**: Actix-web server scaffold (`src/bin/server.rs`) removed. Dependencies cleaned up from Cargo.toml.
- **Learnings**: Don't add API layer until there are consumers. CLI-only is sufficient for now.

### Resolved: Engine module extraction (2026-06-04)
- **Resolution**: Engine commands extracted from `cli/mod.rs` into `src/engine/` module with own binary (`bt-engine`).
- **Learnings**: Clear separation between Python-wrapping CLI and native Rust engine reduces confusion and Docker indirection.

### Resolved: Strategy generation + parquet persistence (2026-06-08)
- **Resolution**: `Strategy` entity, strategy_generator application module, `ParquetStrategyRepository`, and `strategy create` CLI command added. Pipeline persists output to partitioned parquet files.
- **Learnings**: Partitioning by (instrument, mms_id) mirrors backtest constraints and enables partition pruning.

### Resolved: Application layer introduction (2026-06-07)
- **Resolution**: `src/application/` module added with data fetch and generation use cases. `engine/` CLI handlers kept thin.
- **Learnings**: A dedicated application layer keeps CLI handlers thin and use-case logic reusable across binaries.

### Resolved: Domain metric functions (2026-06-24)
- **Resolution**: `max_draw_down()` and `annual_operation_count()` added as pure free functions in `src/domain/`. `filter_strategies()` + `GlobalStrategyFilter` added to `src/domain/strategy.rs`.
- **Learnings**: One file per metric keeps each independently readable and testable.

### Resolved: Real metrics in the strategy generator (2026-06-28)
- **Resolution**: `create_strategies()` computes real `max_draw_down`, `global_roi` → `annual_roi`, and `annual_operation_count` from each strategy's referenced operation points. Parallelized with rayon. `strategy generate` command removed.
- **Learnings**: Two-stage filtering reduces wasted computation. Rayon `par_iter()` is trivial to add for embarrassingly parallel work.

### Resolved: i64 timestamps + SignalGroup in domain (2026-06-28)
- **Resolution**: All timestamps converted to `i64`. `SignalGroup` moved to `domain::signal_group` with embedded `indicator_id`.
- **Learnings**: Domain concepts belong in the domain layer, even if they're cached in infrastructure.

### Resolved: Single-pass performance metrics (2026-08-31)
- **Resolution**: `global_roi`/`max_draw_down`/`annual_roi` combined into `performance_metrics()` with a defensive chronological sort owned by the function.
- **Learnings**: Curve-coupled metrics belong together in one pass; order-dependent metrics must not trust caller ordering.

### Resolved: In-memory repos removed (2026-08)
- **Resolution**: `src/infrastructure/in_memory_repos.rs` deleted; tests use `domain::fixtures` builders and `tempfile`-based adapter tests.
- **Learnings**: Fixtures + real-file tests under temp dirs cover the same needs with less code to maintain.

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
