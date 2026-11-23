<!-- Context: project-intelligence/notes | Priority: high | Version: 1.12 | Updated: 2026-11-14 -->

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
| DbConfig is hardcoded per environment | Host (`host.docker.internal`), ports, and credentials cannot be overridden by env vars | Medium | Fine for the Docker workflow; add env overrides if the engine must target arbitrary PG instances |
| Engine CLI handlers are only partly unit-tested | `create.rs` tests its manifest lookup and `create_loop.rs` its failed-cell formatting; the batch logic it calls is tested in `application::strategy_batch`; the `execute` orchestration itself needs PostgreSQL and is untested | Low | Logic is pushed into tested application/infrastructure functions; add handler tests if handlers grow |
| Parquet write cost at scale | Partition append = full file rewrite; `max_strategy_id()` scans all partitions | Low | Fine at current scale; rethink if partition counts grow by orders of magnitude |
| Stored strategies are not invalidated by logic, threshold or data changes | The manifest records only the `(instrument, mms_id, indicator_id)` triple, so after a change to signal, indicator, metric or validation logic, to `GlobalStrategyFilter::default()`, or to a cell's source data in PostgreSQL, `create`/`create_loop` still skip the cell and its stale strategies stay. Already happened once: every strategy persisted before the entry-timing fix ("enter signals on the next resampled point") selects different operation points than the engine does now | Medium | Reset the whole environment directory (see Gotchas). The real fix, recording an engine/logic/threshold version in the manifest, is not planned yet |

### Technical Debt Details

**Exit-date ordering in metrics**
*Priority*: High
*Impact*: `performance_metrics()` sorts operation points by entry timestamp and compounds in that order. Real trades close at `exit_date`, which is not necessarily in entry order. ROI and max draw down are approximations until this is modeled.
*Root Cause*: The domain `OperationPoint` has no `exit_date` field; the compounding curve was built on entry timestamps.
*Proposed Solution*: Add `exit_date` to `OperationPoint` (DB + domain + adapter), sort/compound on exit order.
*Effort*: Medium
*Status*: Known — required before production use (also recorded in `notes.md`)

**~~Random placeholder signal timestamps~~ (RESOLVED)**
*Priority*: ~~High~~ Resolved
*Resolution*: `create_signal_groups()` now computes real indicator outputs via `create_indicator_output()` (14 indicators) and derives signals via zero-cross / threshold / crossover semantics in `domain/signal/`. All 14 indicators, MACD included, compute real outputs.

**~~Engine module has no unit tests~~ (PARTIALLY RESOLVED)**
*Resolution*: The logic the engine handlers call is tested: the `application` use cases (signal generation, strategy generation, the strategy runner, the `create_loop` batch), the `infrastructure` parquet adapters, manifest, persistence and logging, and the helpers in `engine/strategy`. Each handler's `execute` orchestration needs PostgreSQL and remains untested — acceptable while the handlers stay thin.

**~~Missing HTTP API layer~~ (DEFERRED — deliberate)**
*Status*: Deferred (YAGNI). Actix-web scaffold removed 2026-06-04. Re-add when API consumers exist.

## Open Questions

| Question | Stakeholders | Status | Next Action |
|----------|--------------|--------|-------------|
| Should engine commands be extended with strategy delete/update? | Arman Mojaver | Open | Add commands as needed |
| When should exit-date-aware metrics land? | Arman Mojaver | Open | Required before production use of ROI/DD figures |

## Known Issues

| Issue | Severity | Workaround | Status |
|-------|----------|------------|--------|
| ROI/DD approximate due to entry-order compounding | High | None — model limitation | Known |

## Insights & Lessons Learned

### What Works Well
- **`build -> parse -> execute` pattern** — The one binary, `bte`, follows it at every command level. Simple, predictable, easy to test.
- **Pure validators as free functions** — No object construction needed. Compose well in `combined_validation::validate_all()`.
- **Pure metric functions** — `performance_metrics()` (single pass: global ROI + max draw down, owns the defensive chronological sort) and `annual_operation_count()`. Independently unit-tested, no I/O.
- **Typed indicator parameters + dispatch** — Adding an indicator is mechanical: variant + compute fn + dispatch arm + tests. The `validate_period!` / `empty_oscillator!` / `indicator_values!` macros keep the 14 implementations uniform.
- **Signal semantics as a separate concept** — `IndicatorOutput` (what the indicator computed) is deliberately separate from `SignalGroup` (when to trade). Oscillator/threshold/crossover semantics cover all 25 indicators with three small modules.
- **Single-binary engine** — The engine is the entire CLI surface (`bte`); data management and the iteration table belong to the companion Python service, accessed there directly.
- **Application layer** — `run_strategies()` and the `strategy_batch` use case keep the `engine/` CLI handlers thin.
- **Parquet partitioning by (instrument, mms_id)** — Mirrors the single-instrument/single-MMS backtest constraint; enables partition pruning.
- **Processed manifest** — Idempotency ledger makes `create`/`create_loop` safely re-runnable.
- **Batch parallelism in create_loop** — Load each table once (`rayon::join`), index in memory, compute cells with `par_iter`, persist in one batch. Scales without changing the per-cell logic.

### What Could Be Better
- **Exit-date modeling** — The biggest correctness gap (see Technical Debt).

### Lessons Learned
- **Keep context files in sync with code** — After major refactors, update `.opencode/context/` immediately. Outdated context misleads AI agents. (This file was last aligned with the code on 2026-11-13.)
- **Delete dead code early** — Removing the server scaffold and later the in-memory repos saved maintenance and eliminated misleading references.
- **One file per metric, except coupled metrics** — `annual_operation_count.rs` stands alone; `global_roi`/`max_draw_down`/`annual_roi` live together in `performance_metrics.rs` because they share one compounding curve (single pass).
- **Idempotency belongs at the boundary** — Dedup by `indicator_id` at the parquet partition AND a manifest ledger at the command level: neither alone was sufficient.

## Patterns & Conventions

### Code Patterns Worth Preserving
- **`build -> parse -> execute`** — `bte` follows this 3-step pattern. Lives in `engine/mod.rs` and `engine/strategy/mod.rs`.
- **Pure validation functions** — `src/domain/validation/*.rs`. Each is a free function taking slices, returning `Result`. Composable.
- **Pure metric functions** — `src/domain/performance_metrics.rs` (single-pass `PerformanceMetrics` + `annual_roi` method), `src/domain/annual_operation_count.rs`.
- **Indicator macros** — `validate_period!`, `empty_oscillator!`, `empty_crossover!`, `indicator_values!` in `src/domain/indicators/macros.rs`.
- **`GlobalStrategyFilter::default()`** — Encodes the baseline quality bar in code (DD≤20%, ROI≥5%, ops≥10/yr). Override per-call, default for convenience.
- **Fixture module** — `domain::fixtures.rs` provides reusable test data across all test modules. Single source of truth.
- **Application orchestration** — `src/application/{name}.rs` — one file per use case. Keeps `engine/` thin.
- **Parquet partition layout** — `instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. Matches backtest constraints.
- **Parquet writes** — the manifest is written atomically (temp file + rename); a partition is read, deduped and rewritten in place (intentional: strategies can be rebuilt from PostgreSQL).

- **README stability policy** — The README contains only stable information: abstract domain concepts, CLI surface, data contracts (parquet schema), formulas, tools, and external dependencies. NO function/struct/file names, no folder trees, no test counts — anything that moves with the code lives in these context files, not the README. Rule of thumb: if a code change would force a README edit, that content belongs in `.opencode/context/` instead.

### Gotchas for Maintainers
- **One binary to install** — After changes under `src/engine/`, run `cargo install --path . --bin bte --force`.
- **Python companion is a separate repo** — Data CRUD and the iteration table live in [Backtesting-4.0](../Backtesting-4.0) (Python/Django). The Rust engine talks only to PostgreSQL and the Parquet directory; it has no Python or Docker dependency.
- **Parquet dirs are environment-scoped** — `/app/parquet_files/{env}/strategy` (partitions) and `/app/parquet_files/{env}/processed.parquet` (manifest). Host-mounted via `PARQUET_FILES_PATH`.
- **DB config is hardcoded per environment** — `host.docker.internal` with ports 5432/54320/54321 for production/development/testing. No env override.
- **Metrics are entry-order approximations** — See Technical Debt (exit dates) before quoting ROI/DD numbers.
- **A logic, threshold or data change needs a Parquet reset** — After changing signal, indicator, metric or validation logic, `GlobalStrategyFilter::default()`, or a cell's source data in PostgreSQL, delete the whole `/app/parquet_files/{env}` directory (host: `$PARQUET_FILES_PATH/{env}`) and run `bte strategy create_loop`. The same procedure recovers a corrupt partition. It also removes any manifest temp file left by a killed run. Strategy IDs restart at 1 (`first_persist_starts_ids_at_one`), and only that environment is reset. Never delete half of it: without the manifest, each partition keeps its existing row per `indicator_id` and drops the fresh one, so the old strategies win (`re_persisting_a_cell_keeps_original_ids`); without the partitions, or after deleting one corrupt partition, the manifest still lists their indicators, so nothing is recomputed and those strategies are lost.

## Active Projects

| Project | Goal | Owner | Timeline |
|---------|------|-------|----------|
| Indicator computation module | 25 indicators as pure domain functions + dispatch + macros | Arman Mojaver | Complete |
| Real signal generation | Indicator outputs → SignalGroup via oscillator/threshold/crossover semantics | Arman Mojaver | Complete |
| Processed manifest + idempotency | Ledger of (instrument, mms_id, indicator_id) triples; atomic writes | Arman Mojaver | Complete |
| create_loop batch pipeline | All cells: load once, parallel compute, batch persist | Arman Mojaver | Complete |
| CI workflow | fmt + clippy -D warnings + rustdoc -D warnings + cargo-machete + tests (nextest) + cargo-deny + Docker build/push (toolchain from `rust-toolchain.toml`, 1.98.1) | Arman Mojaver | Complete |
| Exit-date-aware metrics | Compound on exit order, not entry order | Arman Mojaver | Pending — high priority |

## Archive (Resolved Items)

Moved here for historical reference. Current team should refer to current notes above.

### Resolved: MACD compute (2026-10-12)
- **Resolution**: MACD landed in `8c5f4e2` as an Oscillator-kind output (the MACD line, fast MA minus slow MA, each EMA or SMA), which removed the `UnsupportedIndicator` path. **Superseded 2026-11-02**: MACD is a crossover of the MACD line and its signal line; see the next entry.
- **Learnings**: The 'no signal line' reasoning was wrong. The signal period is an engine constant, so the crossover needed no schema change.

### Resolved: MACD is a crossover (2026-11-02)
- **Resolution**: `macd::compute` returns crossover buffers: the MACD line against its `SMA(9)` signal line, with the period fixed in code (`SIGNAL_N`, MQL5 `InpSignalSMA = 9`). The `Crossover` signal kind has its first producer. See the decisions log entry of the same name.
- **Learnings**: The owner's intended semantics are a crossover; the Oscillator shipped in `8c5f4e2` did not match them.

### Resolved: JSON parsing via serde (2026-10-12)
- **Resolution**: The PG adapter deserializes indicator parameters with `serde_json`, with `deny_unknown_fields` on its JSON structs. The `json_extract_*` string scanners are gone. It landed in `eb60dbe`.
- **Learnings**: Hand-scanned JSON panicked on a miss; serde turns schema drift into an error at the boundary.

### Resolved: Real signal generation (2026, prior to 2026-09-03 audit)
- **Resolution**: `create_signal_groups()` computes real indicator outputs (`create_indicator_output`, 14 indicators) and derives signals via `domain/signal/` semantics (oscillator zero-cross, threshold crossing, crossover diff). Replaces the earlier random-timestamp placeholder implementation. MACD was unsupported at the time; it is resolved (see MACD compute above).
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
