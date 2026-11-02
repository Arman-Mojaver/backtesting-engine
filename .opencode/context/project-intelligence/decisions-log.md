<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.14 | Updated: 2026-11-02 -->

# Decisions Log

> Record major architectural and business decisions with full context. This prevents "why was this done?" debates.

## Quick Reference

- **Purpose**: Document decisions so future team members understand context
- **Format**: Each decision as a separate entry
- **Status**: Decided | Pending | Under Review | Deprecated

## Decision Template

```markdown
## [Decision Title]

**Date**: YYYY-MM-DD
**Status**: [Decided/Pending/Under Review/Deprecated]
**Owner**: [Who owns this decision]

### Context
[What situation prompted this decision? What was the problem or opportunity?]

### Decision
[What was decided? Be specific about the choice made.]

### Rationale
[Why this decision? What were the alternatives and why were they rejected?]

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| [Alt 1] | [Pros] | [Cons] | [Why not chosen] |
| [Alt 2] | [Pros] | [Cons] | [Why not chosen] |

### Impact
**Positive**: [What this enables or improves]
**Negative**: [What trade-offs or limitations this creates]
**Risk**: [What could go wrong]

### Related
- [Links to related decisions, PRs, issues, or documentation]
```

---

## Decision: Redis-Backed Signal Groups Cache

**Date**: 2026-06-08
**Status**: Deprecated
**Owner**: Arman Mojaver

### Context
The backtesting engine computes signal groups (mapping of indicator IDs to long/short signal timestamps) during each CLI invocation. For large indicator sets, this computation is non-trivial and is repeated on every invocation even when inputs have not changed. There was no temporary storage layer between CLI calls — data was either in PostgreSQL (permanent) or computed from scratch on each run.

### Decision
Add a dedicated Redis container to the Docker Compose stack and introduce a `redis` top-level command on both `ct` and `bt-engine` binaries. The cache uses a single Redis key (`signal_groups`) via a `SIGNAL_GROUPS_KEY` constant. The schema is enforced at the Rust type level — it is impossible to write a mismatched type to the key.

### Rationale
- Redis is the industry standard for ephemeral cache storage — well-understood, minimal overhead
- Key constant prevents serialization drift between cache writes and reads
- Synchronous API (no async/tokio) matches the existing codebase pattern
- Hard fail on connection errors (no silent cache misses) surfaces problems immediately
- Single Redis key with no TTL keeps the design simple — explicit `--clean` for invalidation

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| In-memory cache in the CLI process | No external dependency | Lost between invocations, no cross-invocation benefit | Doesn't solve the core problem |
| File-based cache (JSON on disk) | No Redis dependency | Race conditions, no atomic operations, manual cleanup | Weaker guarantees than Redis |
| PostgreSQL `LISTEN`/`NOTIFY` | Reuses existing PG connection | Heavy for ephemeral cache, schema migration needed | Wrong tool for the job |
| Async Redis with tokio | Future-proof, async ecosystem | Introduces tokio dependency, doesn't match existing sync codebase | Premature — no async needed yet |

### Impact
**Positive**: Eliminates redundant signal group computation across invocations; typed cache prevents serialization bugs; simple CLI interface for operators
**Negative**: Adds Redis as a Docker dependency; port `6379` must be available
**Risk**: If Redis is down, CLI commands hard-fail — no silent fallback to recomputation (by design)

### Related
- PRD: `.opencode/.tmp/prd/2026-06-08-redis-signal-groups-cache.md`
- Implementation: `src/infrastructure/redis.rs`, `src/engine/redis.rs`, `src/cli/redis.rs`
- Docker: `docker-compose.yaml` — `redis` service

---

## Decision: Remove Redis

**Date**: 2026-08-19
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The Redis-backed signal groups cache (`src/infrastructure/redis.rs`, the `redis` Docker Compose service, and the `ct redis` / `bt-engine redis` CLI commands) was built to avoid recomputing signal groups across CLI invocations. It was never wired into the engine's compute path — it existed only as manual operator tooling. In-memory memoization (the `cached` crate) was added directly to `create_signal_group` in commit `1387bcc`, keyed by `(indicator_id, instrument)`, which provides actual compute-path caching with no external service.

### Decision
Remove Redis entirely: the `redis` Cargo dependency, `src/infrastructure/redis.rs`, `src/cli/redis.rs`, `src/engine/redis.rs`, the `redis` Docker Compose service and `redis-data` volume, and all related documentation.

### Rationale
- In-memory memoization (`cached`) covers the real need — redundant computation within a run
- Redis added operational overhead (extra service, port, dependency) without being integrated into the compute path
- Fewer services = simpler Docker setup and lower maintenance
- YAGNI — re-introduce Redis only if cross-process caching becomes a real requirement

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep Redis | Available for future cross-process caching | Unused in compute path; extra service/dep/port | YAGNI; re-add when needed |
| Wire Redis into the compute path | Cross-invocation caching | Adds I/O + failure modes to the hot path | In-memory `cached` memoization already solves redundant compute |

### Impact
**Positive**: Simpler infra (no `redis` service/volume), fewer dependencies, smaller binary
**Negative**: No cross-process persistence of signal groups (recomputed per process)
**Risk**: Low — no active consumer of Redis; memoization covers the compute path

### Related
- Commit `1387bcc` (feat: add cache to signal group generation) — introduced `cached` memoization
- Supersedes: "Redis-Backed Signal Groups Cache" (2026-06-08)
- Implementation: removed `src/infrastructure/redis.rs`, `src/cli/redis.rs`, `src/engine/redis.rs`, and the `redis` service in `docker-compose.yaml`

---

## Decision: Strategy Generation + Parquet Persistence

**Date**: 2026-06-08
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The engine could validate data but produced no persisted output. To exercise the full pipeline end-to-end (and to give downstream consumers something to read), we needed a way to materialize `Strategy` records and store them in a queryable, partitioned format. PostgreSQL was already the source for raw inputs (indicators, operation points, resampled points) but not the right home for large, append-heavy analytical output.

### Decision
Add a `Strategy` domain entity and a `strategy generate` command path that (a) fetches inputs from PostgreSQL, (b) validates them, (c) generates `Strategy` records, and (d) writes them to **partitioned parquet files** via `ParquetStrategyRepository`. Partition layout: `{base}/instrument={inst}/money_management_strategy_id={mms}/strategies.parquet`. A second `generate` mode produces purely synthetic strategies across `(instrument × mms_id)` partitions with no DB dependency. Base directory is environment-scoped (`/app/parquet_files/{env}/strategy`) and overridable via `--parquet-dir` for running outside Docker.

### Rationale
- Parquet is the standard columnar format for analytical workloads — efficient compression (ZSTD), schema-aware, native to the Arrow/Polars ecosystem
- Partitioning by `(instrument, mms_id)` mirrors the single-instrument/single-MMS backtest constraint, enabling efficient per-partition reads
- Environment-scoped base dir keeps production/development/testing outputs isolated
- A DB-free `generate` mode enables reproducible test fixtures (optional `seed`) without provisioning a database

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Write strategies back to PostgreSQL | Transactional, queryable via SQL | Write-heavy analytical load on OLTP DB; schema migrations | Wrong workload for an OLTP store |
| CSV files | Zero dependencies | No compression, no schema, no columnar reads | Poor for analytical output |
| Single parquet file (no partitioning) | Simpler | Full-file scans; can't prune by instrument/mms | Loses partition pruning benefit |

### Impact
**Positive**: End-to-end pipeline now produces persisted, queryable output; partition pruning enables efficient downstream reads; reproducible synthetic generation for testing
**Negative**: Adds `arrow` + `parquet` dependencies; parquet files require a reader (not human-readable)
**Risk**: Partition explosion if many (instrument, mms) combinations are generated with small counts

### Related
- Implementation: `src/domain/strategy.rs`, `src/application/strategy_generator.rs`, `src/infrastructure/strategy_adapter.rs`, `src/engine/strategy.rs`
- Config: `src/config/environment.rs` — `parquet_base_dir()`
- Docker: `docker-compose.yaml` — `${PARQUET_FILES_PATH}:/app/parquet_files` volume

---

## Decision: Introduce an Application Layer

**Date**: 2026-06-07
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The `engine/` CLI handlers were growing: `strategy create` needed to fetch four datasets from PostgreSQL, validate them, generate strategies, and write parquet. Putting all of that orchestration inside `engine/strategy.rs` would couple CLI parsing logic to infrastructure adapters and domain generation logic, making both harder to test and reuse.

### Decision
Add a new `src/application/` module as the orchestration layer between `domain/` and `infrastructure/`. It owns use-case-level coordination: `strategy_data.rs` (fetch + assemble inputs from PG) and `strategy_generator.rs` (produce `Strategy` records from inputs). `engine/` CLI handlers stay thin — they parse args, call application functions, and report results.

### Rationale
- Keeps `engine/` focused on CLI concerns (parse, dispatch, report) — not data assembly
- Application functions are reusable across `ct` and `bt-engine` binaries
- Mirrors the ports-and-adapters boundary already established between domain and infrastructure
- Application layer is the natural home for "use cases" that combine multiple adapters

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep orchestration in `engine/` | One less module | Couples CLI to infrastructure; harder to test | Violates separation of concerns |
| Put orchestration in `domain/` | Co-located with entities | Domain would depend on infrastructure | Breaks ports-and-adapters |
| Put orchestration in `infrastructure/` | Near adapters | Infrastructure orchestrating domain is inverted | Wrong dependency direction |

### Impact
**Positive**: Thin CLI handlers; reusable use-case functions; clear three-tier separation (domain → application → infrastructure)
**Negative**: One more module to navigate
**Risk**: Application layer could accumulate God-functions if not kept focused — mitigate by one file per use case

### Related
- Implementation: `src/application/mod.rs`, `src/application/strategy_data.rs`, `src/application/strategy_generator.rs`
- Consumer: `src/engine/strategy.rs`

---

## Decision: Performance Metrics as Pure Domain Functions

**Date**: 2026-06-24
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The `Strategy` entity carries performance metrics (`annual_roi`, `max_draw_down`, `annual_operation_count`). We needed to compute `max_draw_down` and `annual_operation_count` from operation points. The question was where these functions should live and what shape they should take.

### Decision
Implement `max_draw_down()` and `annual_operation_count()` as **pure free functions** over `&[OperationPoint]` in `src/domain/` (one file each: `max_draw_down.rs`, `annual_operation_count.rs`). No methods on structs, no I/O, no frameworks. Each is unit-tested in its own file. `filter_strategies()` follows the same pattern over `Vec<Strategy>` + `&GlobalStrategyFilter`.

### Rationale
- Pure functions over slices are maximally composable and testable — no object construction, no mocks
- Keeps the domain dependency-free (consistent with existing validation functions)
- One file per metric makes each independently readable and testable
- `GlobalStrategyFilter` implements `Default` so the baseline quality bar is encoded in code, not config

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Methods on `OperationPoint` / `Strategy` | OO-style discoverability | Requires construction; harder to compose over slices | Less flexible |
| Metrics in `application/` | Near orchestration | Couples metrics to use-case layer; not reusable as domain logic | Wrong layer |
| Single `metrics.rs` file | One place to look | Large file; harder to navigate individual metrics | Loses per-metric focus |

### Impact
**Positive**: Metrics are independently unit-tested, reusable anywhere, and dependency-free; `GlobalStrategyFilter::default()` documents the baseline quality bar in code
**Negative**: One file per metric adds files to `domain/`
**Risk**: None significant — pattern matches existing validation functions

### Related
- Implementation: `src/domain/max_draw_down.rs`, `src/domain/annual_operation_count.rs`, `src/domain/strategy.rs` (`filter_strategies`)
- Consumer: `Strategy` entity stores the computed values

---

## Decision: Extract Engine as Standalone CLI Module

**Date**: 2026-06-04
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The original `cli::mod.rs` contained a mixed bag of commands: some forwarded to a Python CLI via Docker (`docker exec api bt ...`), and some were meant for the Rust engine. The `engine` subcommand was a catch-all that forwarded arbitrary args to `docker exec engine bt-engine ...`. This made the CLI confusing — users had to know which subcommands went to which backend.

### Decision
Extract Rust engine CLI commands into their own `engine/` module with its own `EngineCommand` enum, `build_cli()`, `parse()`, and `execute()`. Compile as a separate binary (`bt-engine`) that runs directly without Docker forwarding.

### Rationale
- Clear separation of concerns: the `bt` binary wraps Python the Python CLI; `bt-engine` is the native Rust engine
- Removes indirection through Docker exec for engine commands
- Each binary has its own command tree — no more catch-all `engine` subcommand
- Follows the same `build -> parse -> execute` pattern already established in `cli/mod.rs`

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep engine as subcommand of `ct` | Single binary, minimal change | Mixed Python/Rust routing, confusing `docker exec engine` indirection | Makes CLI architecture unclear |
| Single `ct` binary with internal dispatch | Single entry point | Over-complicated dispatch logic, tight coupling | Engine and CLI have different deployment needs |

### Impact
- **Positive**: Cleaner architecture, `bt-engine` runs directly, no Docker dependency for engine commands
- **Negative**: Two binaries to maintain and install
- **Risk**: Users must install both binaries; solved by `cargo install --all` or separate install scripts

### Related
- Commit `1f646c9` (cli: add bt-engine cli structure)
- Commit `b3930c6` (cli: add "bt-engine strategy" command)

---

## Decision: Remove Actix-web HTTP Server Scaffold

**Date**: 2026-06-04
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The initial project scaffold included an Actix-web HTTP server (`src/bin/server.rs`) with a single health-check endpoint (`/health` -> `"pong"`). As the project evolved, all actual functionality was CLI-based (backtesting validation and engine commands). The HTTP server was dead code — it was never wired to any real functionality.

### Decision
Remove `src/bin/server.rs` and all Actix-web dependencies from `Cargo.toml`. The project is CLI-only for now.

### Rationale
- Eliminates dead code and its maintenance burden
- Simplifies `Cargo.toml` dependencies (removed `actix-web`, `actix-rt`, `serde`, `serde_json`, `tokio`, `config`, `deadpool-postgres`, `tokio-postgres`, `rust_decimal`, `chrono`)
- Reduces binary size and compile time
- HTTP endpoints can be added later when there's an actual use case and API design

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep server scaffold | No work to remove | Dead code, misleading to newcomers, dependency burden | Clean code principle: delete dead code |
| Rewrite server with simple framework | Lightweight alternative | Premature — no API consumers exist | YAGNI — not needed until API is designed |

### Impact
- **Positive**: Faster builds, smaller dependency footprint, clearer project scope
- **Negative**: HTTP endpoints must be re-created from scratch when needed
- **Risk**: None — no consumers depended on the health check

### Related
- Commit `1f646c9` (cli: add bt-engine cli structure)

---

## Decision: Dual Binary Layout (ct + bt-engine)

**Date**: 2026-06-04
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The `Cargo.toml` originally defined a single `[[bin]]` entry for the `ct` binary. After extracting the engine commands, we had two separate command trees that needed their own entry points.

### Decision
Define two `[[bin]]` entries in `Cargo.toml`:
- `ct` at `src/main.rs` — wraps the Python CLI (`cli/`)
- `bt-engine` at `src/bin/bt-engine.rs` — native Rust engine (`engine/`)

### Rationale
- Each binary has a focused responsibility
- Both share the same library crate (`backtesting-engine`) for shared types
- Users can install just what they need: `cargo install --path . --bin ct` or `cargo install --path . --bin bt-engine`
- Follows Cargo's native multi-binary pattern

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Single binary with subcommand dispatch | Single install | Mixed responsibility, longer build | Two binaries already in use |
| Separate crates | Fully isolated | Duplicate shared types, workspace complexity | Overkill — shared domain types benefit from single crate |

### Impact
- **Positive**: Clean separation, shared library, flexible install
- **Negative**: Two binaries to build and version
- **Risk**: Dependency drift between binaries (mitigated by shared lib crate)

### Related
- `Cargo.toml` — `[[bin]]` entries
- `src/main.rs`, `src/bin/bt-engine.rs`


## Decision: i64 Timestamps Across the Domain

**Date**: 2026-06-27
**Status**: Decided
**Owner**: Arman Mojaver

### Context
All timestamps in the domain (`OperationPoint.timestamp`, `ResampledPointD1.timestamp`, signal timestamps) were `i32`. While sufficient for dates after 1970, `i32` is fragile for far-future dates and inconsistent with the broader Rust ecosystem which favours `i64` for epoch seconds. The PG adapter was reading `i32` columns directly.

### Decision
Convert all timestamp fields to `i64` across the domain: `OperationPoint`, `ResampledPointD1`, `SignalGroup` signal vectors, `OperationPoints` lookup maps, and validation HashSets. The PG adapter casts the `i32` DB column to `i64` on read (`row.get::<_, i32>("timestamp") as i64`).

### Rationale
- `i64` is the Rust ecosystem standard for epoch seconds (matches `chrono`, `time`, `std::time`)
- Future-proofs against Y2038 concerns and far-future backtests
- Consistent type across domain boundaries (no mixed `i32`/`i64` comparisons)

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep `i32` | No code change | Fragile, inconsistent with ecosystem | Technical debt |
| `u64` | No negative timestamps | Breaks arithmetic (subtraction can underflow) | `i64` is safer |

### Impact
**Positive**: Consistent, future-proof timestamps; matches ecosystem conventions
**Negative**: PG adapter must cast on read
**Risk**: None — internal type change, no external API impact

### Related
- Commit `514a846` (chore: convert all timestamps to i64)
- Implementation: `src/domain/operation_point.rs`, `src/domain/resampled_point.rs`, `src/infrastructure/pg_adapter.rs`

---

## Decision: SignalGroup Moved to Domain + Embeds indicator_id

**Date**: 2026-06-27
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`SignalGroup` previously lived in `infrastructure::redis` as a HashMap keyed by indicator ID (`SignalGroups = HashMap<i32, SignalGroup>`). The `SignalGroup` struct itself had no `indicator_id` field — the association was implicit via the HashMap key. This meant the domain layer couldn't reference `SignalGroup` without depending on infrastructure, and the HashMap structure was lost when serialized to Redis (JSON array).

### Decision
Move `SignalGroup` to `domain::signal_group`, add an `indicator_id: i32` field to the struct, and replace `SignalGroups` (HashMap) with `Vec<SignalGroup>`. The Redis cache now stores/reads `Vec<SignalGroup>` directly. Added `operation_count()` method and `filter_signal_groups_by_annual_ops()` free function to the domain module.

### Rationale
- `SignalGroup` is a domain concept (a set of trade signals for an indicator) — it belongs in the domain layer
- Embedding `indicator_id` makes the association explicit and survives serialization (no HashMap key loss)
- `Vec<SignalGroup>` is simpler than `HashMap` and matches the Redis JSON array representation
- Domain-level `filter_signal_groups_by_annual_ops()` enables pre-filtering signal groups before the expensive strategy-creation stage

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep `SignalGroup` in infrastructure | No domain change | Domain can't reference it; HashMap key lost on serialize | Wrong layer |
| Keep HashMap, add `indicator_id` field | Redundant key | Duplicates the ID in key and field | Redundancy |

### Impact
**Positive**: Domain owns `SignalGroup`; explicit ID association; pre-filtering reduces downstream work; simpler Redis schema
**Negative**: `Vec` lookup by ID is O(n) vs HashMap O(1) — acceptable (signal groups are iterated, not looked up by ID)
**Risk**: None significant

### Related
- Commit `a367327` (domain: embed indicator_id in SignalGroup, adapt redis to new structure)
- Implementation: `src/domain/signal_group.rs`, `src/infrastructure/redis.rs`, `src/engine/redis.rs`

---

## Decision: Compute Real Metrics in the Strategy Generator

**Date**: 2026-06-28
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The strategy generator previously filled `annual_roi`, `max_draw_down`, and `annual_operation_count` with **random placeholder values**. The pure domain metric functions (`max_draw_down()`, `annual_operation_count()`) existed but were not wired in. Downstream consumers reading parquet output saw synthetic numbers, not real performance. This was tracked as high-priority technical debt.

### Decision
Rewrite the strategy generator (`application::strategy_generator`) to compute real metrics. `create_strategies()` iterates signal groups (parallelized with **rayon** `par_iter()`), and for each signal group `create_strategy()`:
1. Resolves long/short operation points by timestamp lookup
2. Sorts all ops by timestamp
3. Computes `max_draw_down(&all_ops)`, `global_roi(&all_ops)` → `annual_roi(start, end, global)`, and `annual_operation_count(count, start, end)`
4. Builds a `Strategy` with the computed metrics

Added two new domain metric functions: `global_roi()` (total ROI compounding against cummax) and `annual_roi()` (annualizes global ROI via compounding).

### Rationale
- Real metrics make parquet output trustworthy for downstream consumers
- Rayon parallelism scales across many signal groups
- Two-stage filtering (signal groups by op count, then strategies by DD + ROI) reduces wasted computation
- `global_roi` + `annual_roi` separation mirrors the `max_draw_down` pattern (pure fn over `&[&OperationPoint]`)

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep random placeholders | No work | Misleading output; high-priority debt | Unacceptable |
| Compute metrics lazily on read | Defers cost | Complex; parquet is columnar | Premature |
| Single-threaded computation | Simpler | Slow for many signal groups | Rayon is trivial to add |

### Impact
**Positive**: Parquet output now carries real, computed metrics; resolves high-priority technical debt; rayon parallelism for performance
**Negative**: Generator now depends on operation-point data (not purely synthetic)
**Risk**: Signal generator (`create_signals`) still produces random timestamps — the metrics are real given the signals, but the signals themselves are random placeholders

### Related
- Commits: `479e3d7` (annual_roi), `cc7ed18` (max_draw_down), `147001a` (operation_count), `5267749` (rayon)
- Implementation: `src/application/strategy_generator.rs`, `src/domain/annual_roi.rs`, `src/domain/global_roi.rs`
- Resolves: "Random placeholder strategy metrics" technical debt (see living-notes.md)

---

## Decision: Remove `strategy generate` Command

**Date**: 2026-06-28
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The `strategy generate` command (both `ct s generate` and `bt-engine strategy generate`) produced purely synthetic strategies with random metrics across `(instrument × mms_id)` partitions. With the move to real-metric computation, the synthetic generator (`generate_random_strategies_bulk`) no longer fit — it had no real operation points to compute metrics from. Keeping it would produce strategies with zero/meaningless metrics.

### Decision
Remove the `generate` subcommand from both `ct` (`src/cli/strategy.rs`) and `bt-engine` (`src/engine/strategy/`). Strategy creation now flows exclusively through `strategy create`, which fetches real data from PG, generates signals, and computes real metrics.

### Rationale
- Synthetic generation with random metrics is no longer useful — the project now computes real metrics
- `strategy create` covers the real use case (DB-backed strategy creation)
- Removes dead code and CLI surface area
- Simplifies the engine strategy module (now a directory: `mod.rs` + `create.rs`)

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep `generate` with zero metrics | Backward compat | Misleading output | Unacceptable |
| Keep `generate` but require DB | Reuses create logic | Redundant with `create` | Duplication |

### Impact
**Positive**: Simpler CLI; no misleading synthetic output; cleaner engine strategy module
**Negative**: Lost ability to generate test fixtures without a DB
**Risk**: Low — `strategy create` is the real path

### Related
- Commit `8475e1a` (chore: remove strategy generate command)
- Implementation: `src/cli/strategy.rs`, `src/engine/strategy/mod.rs`, `src/engine/strategy/create.rs`

---

## Decision: Parquet Append Semantics + Global strategy_id Assignment

**Date**: 2026-06-28
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The `ParquetStrategyRepository` needed to support multiple writes to the same partition without overwriting previous strategies, and strategy IDs needed to be unique across all partitions. The `Strategy` domain struct had a `strategy_id` field, but assigning it in the domain layer would couple the domain to persistence concerns.

### Decision
1. Remove `strategy_id` from the `Strategy` domain struct — IDs are persistence-assigned, not domain-owned.
2. `ParquetStrategyRepository::write()` reads existing rows (if the parquet file exists), appends new rows, and rewrites the whole file.
3. `find_max_strategy_id()` recursively scans ALL partitions under `base_dir` to find the global max ID. New IDs start at `global_max + 1` and are contiguous within a batch.

### Rationale
- Domain stays free of persistence concerns (no `strategy_id` on `Strategy`)
- Append semantics preserve existing strategies across writes
- Global max scan ensures IDs are unique and contiguous across partitions and appends
- Rewriting the whole file is simple and correct (parquet files are immutable; append = rewrite)

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| `strategy_id` on domain struct | Domain owns ID | Couples domain to persistence | Violates ports-and-adapters |
| Append without reading existing | Faster | Loses existing data | Data loss |
| Per-partition ID counter | Faster scan | IDs not globally unique | Ambiguous across partitions |

### Impact
**Positive**: Clean domain; append preserves data; globally unique contiguous IDs
**Negative**: Full file rewrite on each write; recursive scan for max ID (O(all partitions))
**Risk**: Performance degradation with many partitions — mitigate by caching max ID if needed

### Related
- Implementation: `src/infrastructure/strategy_adapter.rs`, `src/domain/strategy.rs`

---

## Decision: Combine Metric Functions into Single-Pass `performance_metrics()`

**Date**: 2026-08-31
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`global_roi()` and `max_draw_down()` looped over the same `&[&OperationPoint]` data with an identical compounding loop (`cumsum = 1 + Σ cummax * result * risk/sl`) — `create_strategy()` paid for two full passes per strategy. The chronological sort that makes the metrics correct lived in the **caller**, so any future caller passing unsorted data would silently get wrong numbers (both metrics are order-dependent). `annual_roi()` was a free function taking a bare `global_roi: f64`, detached from the metrics it derives from.

### Decision
Replace the three metric files (`global_roi.rs`, `max_draw_down.rs`, `annual_roi.rs`) with a single `src/domain/performance_metrics.rs`:
1. `performance_metrics(ops: &[&OperationPoint]) -> PerformanceMetrics` computes both metrics in **one pass** over the shared compounding curve. Each metric is rounded to 2dp independently — outputs are byte-identical to the old functions (`annual_roi` consumes the ROUNDED `global_roi`).
2. The function **owns the defensive chronological sort** (stable `sort_by_key(timestamp)`; ties keep input order). Callers do not need to pass sorted data. `create_strategy()` no longer sorts.
3. `annual_roi` became a **method** on `PerformanceMetrics`: `metrics.annual_roi(start_ts, end_ts)` — `global_roi` is already an attribute.
4. Tests: 35 total — 34 mapped 1:1 from the old suites (same names, nested under `tests::{global_roi, max_draw_down, annual_roi}`) plus 2 new sorting tests (`unsorted_input_computed_in_chronological_order`, `equal_timestamps_preserve_input_order`).

### Rationale
- One pass instead of two over the same data. The loop is serial (loop-carried `cumsum`/`cummax`), so single-pass is the only real win available; rayon already parallelizes across strategies
- Defensive sort closes the silent-wrong-metrics hole: order-dependent metrics must not trust caller ordering
- Struct return is self-documenting and extensible; the method keeps `annual_roi` next to the value it consumes

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep two separate functions | No test churn | Double loop over the same data | The reason for this change |
| Thin wrappers delegating to the combined fn | Backwards compatible | Dead code — only one caller existed | Minimal API surface preferred |
| Merge pre-sorted long/short streams in O(n) instead of sorting | Faster (O(n) vs O(n log n)) | Requires callers to guarantee sorted input — exactly the trust this decision removes; op counts are hundreds, so sort cost is negligible | Rejected by owner: defensive design over micro-optimization |
| Sort stays in the caller | Function stays "pure over given order" | Silent wrong metrics on unsorted input | The hole this decision closes |

### Impact
**Positive**: Halves metric iteration cost; ordering invariant enforced at the point of use; `annual_roi` colocated with its input; one test file with 1:1 mapping to the old suites
**Negative**: Breaks the "one file per metric" convention for these three (deliberate — they share one compounding curve); `to_vec()` copy per call for the defensive sort (trivial at hundreds of ops)
**Risk**: None — all 34 old test expectations preserved verbatim and passing

### Related
- Implementation: `src/domain/performance_metrics.rs`, `src/application/strategy_generator.rs`
- Supersedes: the metric-file layout from "Domain metric functions" (2026-06-24) and "Compute Real Metrics in the Strategy Generator" (2026-06-28)

---

---

## Decision: Port Indicators to Rust + Real Signal Generation

**Date**: 2026-07-09 → 2026-07-26 (incremental ports; signal abstractions refactored 2026-07-25)
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`create_signals()` generated `SignalGroup`s by sampling **random timestamps** from resampled points — the signals were placeholders, not derived from indicator logic. Metrics computed from those signals were real, but the strategies themselves were meaningless. This was the highest-priority technical debt item.

### Decision
Port the indicator library to Rust as pure domain functions and derive signals from real indicator mathematics:
1. `src/domain/indicators/` — one file per indicator (AO, AC, Bears, Bulls, CHO, CHV, DPO, Force, OsMa, ROC, TRIX, VROC, RSI), shared moving-average math in `moving_averages.rs`, uniform macros (`validate_period!`, `empty_oscillator!`, `indicator_values!`).
2. `create_indicator_output(parameters, points)` dispatches the typed `IndicatorParameters` enum to a compute function and returns an `IndicatorOutput` — `Oscillator`, `Threshold`, or `Crossover`.
3. `src/domain/signal/` — three small semantics modules convert an output to signals: oscillator zero-cross, threshold crossing, crossover (series diff → zero-cross). `IndicatorOutput::create_signal_group()` routes to the right rule.
4. MACD is **skipped, not aborted**: compute returns `UnsupportedIndicator`; the signal generator's `filter_map` drops it and the run continues.

### Rationale
- Signals must come from indicator mathematics or the whole pipeline is theater
- Three output kinds cover all indicators — signal semantics are a separate, small concept
- Typed parameter enum + dispatch makes adding an indicator mechanical (variant + compute fn + arm + tests)
- Macros keep 14 implementations uniform (period validation, empty-series handling, buffer access)
- Skipping unsupported indicators keeps batch runs resilient — one bad indicator must not kill a 500-indicator run

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep random timestamps | No work | Strategies are meaningless | Unacceptable |
| Compute indicators in the Python companion | Reuses existing code | Cross-process I/O on the hot path; splits the domain | Engine must own its math |
| Abort run on unsupported indicator | Fail-fast purity | One unsupported indicator kills the whole batch | Skip-and-continue is the right trade for batch workloads |

### Impact
**Positive**: Signals are real; the pipeline is honest end-to-end; indicator ports are uniform and testable
**Negative**: MACD strategies are silently absent until implemented (logged via skip)
**Risk**: Silent skips could hide coverage gaps — mitigated by `strategy metadata` reporting

### Related
- Implementation: `src/domain/indicators/`, `src/domain/signal/`, `src/application/signal_generator.rs`
- Resolves: "Random placeholder signal timestamps" technical debt (living-notes.md)
- Commits: `27651c5` (AO) through `d70b518` (VROC), `d8063aa` (signal abstractions), `9af8fb1` (SignalGroup attributes)

---

## Decision: Processed Manifest + Idempotent Partitions

**Date**: 2026-08-07
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Re-running `strategy create` recomputed every indicator and appended duplicate rows to the parquet partitions. There was no record of which `(instrument, mms_id, indicator_id)` work was already done, so batch runs were neither idempotent nor resumable.

### Decision
1. Add `ParquetProcessedManifest` — a single `processed.parquet` ledger under the environment root recording every completed `(instrument, mms_id, indicator_id)` triple with a `processed_at` timestamp. Appends dedup on the triple; writes are atomic (temp file + rename).
2. Make partition writes idempotent: `write_partition` reads existing rows and drops fresh rows whose `indicator_id` already exists — re-runs never duplicate.
3. Add `bt-engine strategy metadata` — a DB-vs-manifest report of done and remaining triples.

### Rationale
- Idempotency at the command level (manifest) AND the output level (partition dedup) — neither alone is sufficient: the manifest says what was attempted, the partition dedup guards the data itself
- Parquet (not a DB table or JSON file) keeps the ledger in the same ecosystem as the output, with the same atomic-write discipline
- Atomic rename prevents a half-written manifest from corrupting the run state
- The metadata command falls out of the ledger for free — operators get a progress report

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Track processed state in PostgreSQL | Transactional | Write-heavy OLTP load; couples engine to DB for run state | Run state belongs with the output |
| JSON/CSV manifest | Human-readable | No schema, no atomic rewrite discipline | Weaker guarantees |
| Partition dedup only (no manifest) | Simpler | Must read every partition to know what's done; no processed_at history | O(partitions) per check; no audit trail |

### Impact
**Positive**: `create`/`create_loop` are safely re-runnable; runs are resumable; operators get progress reporting
**Negative**: Manifest append rewrites the whole ledger file (fine at current scale)
**Risk**: Manifest and partitions can drift if a run crashes between partition write and manifest commit — partition dedup makes the drift self-healing on re-run

### Related
- Implementation: `src/infrastructure/processed_manifest.rs`, `src/infrastructure/strategy_adapter.rs`, `src/engine/strategy/metadata.rs`, `src/application/metadata_report.rs`
- Commit: `18318cf` (parquet: add processed manifest, dedup-on-write, and metadata command)

---

## Decision: `create_loop` Batch Pipeline

**Date**: 2026-08-08 (introduced), refined through 2026-08-30 (adapter extraction, n+1 fix, parallel queries)
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`strategy create` processes one `(instrument, mms_id)` cell per invocation. Running the whole database meant a shell loop: one DB connection set, one fetch, one parquet write per cell — n+1 query patterns and no cross-cell parallelism.

### Decision
Add `bt-engine strategy create_loop`:
1. Load all four source tables **once**, in parallel (nested `rayon::join`), and index them in memory (`resampled_by_instrument`, `long_by_cell`, `short_by_cell`).
2. Derive the cell list from the operation-point indexes (union of long/short cells).
3. Compute every cell with `par_iter`, skipping cells whose indicators are all in the processed manifest.
4. Persist as one batch via `StrategyPersistence`: assign `strategy_id`s sequentially from the global max, write partitions in parallel, commit the manifest once (`append_all`).

### Rationale
- One query per table instead of one per cell — eliminates the n+1 pattern entirely
- Cells are independent — rayon `par_iter` is the natural fit, same as strategies within a cell
- Batch ID assignment + single manifest commit keeps IDs contiguous and the ledger consistent
- Reuses `run_strategies()` unchanged — the per-cell logic is identical to `create`

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Shell loop over `strategy create` | No new code | n+1 queries; serial; per-run process startup | Slow and fragile |
| Query per cell inside the loop | Simple | Still n+1 connections/queries | The reason for this change |
| Single threaded in-process loop | One connection set | Wastes cores; cells are independent | Rayon is trivial here |

### Impact
**Positive**: Whole-database runs are parallel, query-efficient, and idempotent
**Negative**: Peak memory holds all source tables indexed in memory (acceptable — daily candles and trade records are small relative to RAM)
**Risk**: A failure mid-batch leaves some partitions written but the manifest uncommitted — self-healing on re-run via partition dedup

### Related
- Implementation: `src/engine/strategy/create_loop.rs`, `src/application/cell_output.rs`, `src/infrastructure/strategy_persistence.rs`
- Commits: `4bc124c` (create_loop), `a333f6f` (n+1 fix), `ddfb72d` (adapters), `eebac2f` (parallel queries)

---

## Decision: Remove In-Memory Repositories

**Date**: 2026-08-29
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`src/infrastructure/in_memory_repos.rs` implemented all four repository traits for tests. In practice, tests used `domain::fixtures` builders for domain logic and `tempfile`-based tests for the parquet adapters — the in-memory repos had no remaining consumers and duplicated fixture functionality.

### Decision
Delete `in_memory_repos.rs`. Domain tests use fixtures; adapter tests use real files under `tempfile::TempDir`.

### Rationale
- Dead code with maintenance cost and a misleading suggestion of a production in-memory mode
- Fixtures + tempfile tests cover the same needs with less code
- Testing adapters against real parquet files is higher-fidelity than testing against in-memory fakes

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep for future use | Available | Unused; misleading | YAGNI |
| Move to a test-support crate | Organized | Still dead code, now with ceremony | No consumers |

### Impact
**Positive**: Less code; clearer testing story (fixtures for domain, tempfile for adapters)
**Negative**: Re-adding an in-memory mode requires writing it again
**Risk**: None — no consumers

### Related
- Commit: `2bc5df3` (chore: remove in_memory_repos.rs)
- Testing patterns: `src/domain/fixtures.rs`, `src/infrastructure/processed_manifest.rs` tests

---

## Decision: Structured Logging via `log` + `simplelog`

**Date**: 2026-08-29
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Production code used `eprintln!` for progress output. There were no levels, no timestamps, and no way to route logs to a file — the Docker container tailed `logs/bt-engine.log`, which nothing wrote to.

### Decision
Adopt the `log` facade with `simplelog` (term + file writers, `RUST_LOG`-driven levels). Replace all `eprintln!` in production code with level-appropriate `info!`/`warn!`/`debug!`/`error!`. Propagate logs to the Docker setup (compose tails the log file).

### Rationale
- `log` facade keeps the domain free of any logging backend (infrastructure owns `logging::init()`)
- `RUST_LOG` gives operators per-module level control without rebuilds
- File writer feeds the container's log tail; timestamps make batch runs auditable

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep `eprintln!` | No work | No levels/timestamps/file routing | Unacceptable for batch tooling |
| `tracing` + subscriber | Structured spans, ecosystem standard | Heavier dependency; spans unused in a batch CLI | Premature |
| `env_logger` | Simple | File output needs shell redirection | simplelog writes files natively |

### Impact
**Positive**: Leveled, timestamped, file-routed logs; `RUST_LOG=debug` reveals filter-stage counts
**Negative**: One more dependency (`simplelog`)
**Risk**: None significant

### Related
- Implementation: `src/infrastructure/logging.rs`, `docker-compose.yaml` (log tail), `.env.example` (`RUST_LOG`)
- Commits: `ed936ae` (logger), `e8b895d` (propagate to docker)

---

## Decision: CI Workflow (fmt + clippy -D warnings + tests + Docker)

**Date**: 2026-09-03
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The repository had no CI. Formatting, lint, and test discipline existed locally but was not enforced; Docker images were built and pushed by hand via the Makefile.

### Decision
Add `.github/workflows/ci.yml`:
1. `lint-and-test` job on push to `main` (+ workflow_dispatch): pinned Rust **1.94.0** (matching the Dockerfile's `rust:1.94`), `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, with `Swatinem/rust-cache`.
2. `docker` job (gated on lint-and-test): build and push `armanmojaver/backtesting-engine` tagged `latest` + short commit SHA, with GitHub Actions layer cache.

### Rationale
- Zero-warning clippy policy enforced at the gate, not by memory
- Pinning the toolchain to the Dockerfile's Rust version prevents CI/local/image drift
- Docker push only after tests pass — a broken image can never be published
- SHA tags give every image a traceable provenance point

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| No CI | No work | Discipline decays | Unacceptable |
| Tests only (no fmt/clippy gate) | Cheaper | Style/lint drift accumulates | The gate is the point |
| Unpinned toolchain | Always latest | CI/image version drift | Pinning is one line |

### Impact
**Positive**: Every merge to main is formatted, lint-clean, tested, and published with provenance
**Negative**: CI minutes per push; Docker credentials must exist as secrets
**Risk**: None significant

### Related
- Implementation: `.github/workflows/ci.yml`, `Dockerfile` (rust:1.94), `Makefile` (local equivalents)

---

## Decision: Pin Rust Toolchain Exactly Across Local, Docker, and CI

**Date**: 2026-09-04
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The Dockerfile pinned `rust:1.94` (floating patch), CI pinned `1.94.0` exactly (dtolnay/rust-toolchain), and local dev floated on `rustup stable` — three environments with two different pinning semantics. Local would silently drift from CI/Docker on the next stable release. A senior-engineer review suggested `rust-toolchain.toml` with `channel = "1.94"`; investigation showed a floating minor pin preserves the drift problem instead of fixing it.

### Decision
1. Add `rust-toolchain.toml`: `channel = "1.94.0"` (exact), `profile = "minimal"`, `components = ["rustfmt", "clippy"]` — rustup applies it automatically for local dev.
2. Bump all three Dockerfile base tags `rust:1.94` → `rust:1.94.0` so Docker pins the identical version.
3. Exclude `rust-toolchain.toml` from the Docker build context via `.dockerignore`.
4. Add `rustfmt.toml` (`edition = "2021"`, all defaults) as an explicit formatting-policy marker; enforcement remains `cargo fmt --check` in CI.

### Rationale
- Exact pins in all three environments eliminate drift by construction
- Empirical finding (validated with `--network none` containers): official `rust` images run `cargo` through **rustup shims**. A `rust-toolchain.toml` inside the build context triggers a channel-manifest network sync + component download (the image's `minimal` profile lacks clippy) **even when the version matches the installed toolchain**. Excluding the file keeps Docker builds offline-clean; the image tag remains Docker's native pin
- `rustfmt.toml` documents intent; the actual gate is CI's `cargo fmt --check` (unchanged)
- docker-compose `engine` mounts only `src/`, `Cargo.toml`, `Cargo.lock` — the compose flow never sees the toolchain file; only `make bash` (full repo mount) does, costing a one-time ~5s sync per fresh container

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| `channel = "1.94"` (floating minor, as suggested) | Looks simpler | Resolves to latest 1.94.x patch — preserves drift vs CI's exact 1.94.0 | Drift is the problem being fixed |
| Keep `rust-toolchain.toml` in Docker context | Single file visible in image | rustup shim hits the network in every build (manifest + clippy download) even on matching versions | Zero benefit — image tag already pins |
| No toolchain file | Status quo | Local floats; drifts from CI/Docker on next stable | The gap this decision closes |
| `clippy.toml` (empty) | Ceremony | No settings to configure | Rejected as noise |

### Impact
**Positive**: Local, CI, and Docker compile with the identical toolchain; Docker builds stay offline-clean
**Negative**: Version bumps touch three places (`rust-toolchain.toml`, `Dockerfile`, `ci.yml`) — deliberate trade for exactness
**Risk**: Removing the `.dockerignore` exclusion would silently reintroduce the network sync — mitigated by comments in both files and this entry

### Related
- Implementation: `rust-toolchain.toml`, `rustfmt.toml`, `.dockerignore`, `Dockerfile`, `README.md` (toolchain line)
- CI: `.github/workflows/ci.yml` (pins 1.94.0 via dtolnay/rust-toolchain)
- Complements: "CI Workflow" (2026-09-03), which pinned the CI side only

---

## Decision: Deliberate clippy Lint Policy via `[lints]` (pedantic + curated allows)

**Date**: 2026-09-04
**Status**: Decided
**Owner**: Arman Mojaver

### Context
A senior-engineer review suggested `#![warn(clippy::all, clippy::pedantic)]` + `#![warn(missing_docs)]` in `src/lib.rs`. Measured against this codebase: `clippy::all` is warn-by-default (a no-op); lib.rs inner attributes cover only the lib target, not the `ct`/`bt-engine` binaries; `clippy::pedantic` fires 1,429 warnings which CI's `-D warnings` gate (added 2026-09-03) would turn into build errors; `missing_docs` fires 366 warnings on an internal, unpublished crate. As written, the suggestion would break CI.

### Decision
1. Enable pedantic via the `[lints.clippy]` table in `Cargo.toml` — one declaration covering all targets (lib, both binaries, tests) — with a commented allow list.
2. Allow list (each entry commented in `Cargo.toml`): numeric-domain lints (`float_cmp`, `cast_lossless`, `cast_sign_loss`, `cast_precision_loss`, `cast_possible_truncation`, `cast_possible_wrap`); deferred doc lints (`missing_errors_doc`, `missing_panics_doc`, `doc_markdown`); low-signal lints (`must_use_candidate`, `similar_names`, `implicit_hasher`); divisive style lints (`single_match_else`, `manual_let_else`).
3. Test-scoped exemption in `src/lib.rs`: `#![cfg_attr(test, allow(clippy::unreadable_literal))]` — all 672 hits are in test code (exact float literals); the lint stays armed for production code via the plain lib target.
4. Fixed every remaining production violation: `cargo clippy --fix` campaign (~20 files of mechanical style fixes) plus manual fixes — 3 `map_unwrap_or` sites in `create_loop.rs` rewritten as type-annotated `map_or` (dodging a rustfix bug: the `&[]` default loses its coercion site, E0271, which rolls back the whole campaign); `read_rows`/`write_partition` became associated functions; `parse()` takes `&ArgMatches`; `create::execute` takes `&str`; `Compression::ZSTD(ZstdLevel::default())`.
5. `missing_docs` deferred: 366 undocumented public items on an internal crate — permanent warnings train developers to ignore the warning stream. Revisit if the crate is ever published.

### Rationale
- `[lints]` (stable since 1.74) is the correct mechanism — the suggested lib.rs attributes would leave both binaries un-linted
- The allow list is policy, not noise: every entry has a reason tied to this codebase (float math, PG adapter casts, internal crate)
- `f64::midpoint` replacing `(h+l)/2.0` in AO/AC median price is bit-identical for all finite values ≤ `f64::MAX/2` (verified against the std source); all 429 tests pass unchanged
- The rustfix `map_unwrap_or`/`&[]` bug made the 3 sites manual-first; the rest of the campaign then applied cleanly

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Senior suggestion as written (lib.rs attributes) | One file | `clippy::all` no-op; misses both binaries; 1,429 CI errors | Breaks the gate; wrong mechanism |
| pedantic with no allows | Strictest | ~1,400 violations — a refactor project, not a config change | Most hits are domain noise |
| No pedantic (clippy::all only) | Status quo | Misses the ~45 genuine fixes that landed | The campaign was cheap and is done |
| `missing_docs` now | Doc completeness | 366 warnings of noise on an internal crate | Deferred deliberately |

### Impact
**Positive**: pedantic armed across all targets with a documented policy; ~45 genuine fixes landed; production code is pedantic-clean under `-D warnings`
**Negative**: allow list must be revisited as the codebase evolves (re-enable doc lints if published); test literals exempt from `unreadable_literal`
**Risk**: New code could drift into allowed-lint territory — mitigated by per-entry comments and this entry

### Related
- Implementation: `Cargo.toml` (`[lints.clippy]`), `src/lib.rs` (cfg_attr), ~20 src files (fix campaign)
- Complements: "CI Workflow" (2026-09-03) — the `-D warnings` gate now enforces pedantic
- Complements: "Pin Rust Toolchain Exactly" (2026-09-04)

---

## Decision: Toolchain 1.94.0 → 1.98.1 + Dependency Refresh

**Date**: 2026-09-17
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Rust 1.94.0 (pinned 2026-09-04) was four stable releases behind (1.98.1 released 2026-09-01). Dependencies had drifted: arrow/parquet locked at 58.3.0 (latest 60.0.0), `cached` at 2.0.2 (latest 4.0.0), and `time` carried an exact `=0.3.41` pin as a direct dependency despite no direct usage in code (only `std::time` is used; the crate existed solely as simplelog's transitive dependency).

### Decision
1. Bump the exact toolchain pin 1.94.0 → 1.98.1 in all three places (`rust-toolchain.toml`, Dockerfile ×3, ci.yml ×2), per the exact-pin policy.
2. Upgrade major dependencies: arrow/parquet 58 → 60, `cached` 2 → 4.
3. Remove the unused direct `time` dependency (simplelog continues to pull it transitively, now at 0.3.55).
4. `cargo update` for semver-compatible refreshes (clap 4.6.7, anyhow 1.0.104, postgres 0.19.14, log 0.4.34, rustc-hash 2.1.3 + transitive).
5. Fix the three new clippy lints surfaced by the four-version toolchain jump: 2× `map_unwrap_or` (`create.rs`, `strategy_persistence.rs` → `map_or`) and 1× `too_long_first_doc_paragraph` (`test_support.rs` doc restructured).

### Rationale
- Regular toolchain/dependency updates for security and currency (security-patterns dependency policy)
- The exact-pin policy makes bumps a deliberate 3-place change — done atomically
- `time = "=0.3.41"` was a workaround-era pin with no remaining direct consumer; removing it minimizes the dependency count and lets the transitive resolve freely
- arrow 60 / cached 4 MSRVs (1.88 / 1.92) are satisfied by 1.98.1

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Stay on 1.94.0 | Zero effort | Security/currency drift grows | Regular updates are policy |
| Bump toolchain only, keep deps | Smaller change | arrow/cached majors left behind | Deps were part of the goal |
| Keep `time` as direct dep (unpinned "0.3") | Explicit version control | Unused direct dep = misleading manifest | Not directly used; transitive suffices |
| Float pin (`1.98`) | Fewer future bumps | Rejected by the 2026-09-04 exact-pin decision | Drift is the problem the policy fixes |

### Impact
**Positive**: Current toolchain (four releases of improvements), current deps, one fewer direct dependency, zero-warning gate preserved, all 429 tests pass
**Negative**: New toolchain versions can surface new lints (they did: 3, all fixed); future bumps remain 3-place changes
**Risk**: arrow 58→60 spanned two majors — compiled clean here, but future major bumps deserve the same full validation gate

### Related
- Implementation: `rust-toolchain.toml`, `Dockerfile`, `.github/workflows/ci.yml`, `Cargo.toml`, `Cargo.lock`, 3 lint-fix files
- Follows: "Pin Rust Toolchain Exactly" (2026-09-04), "Deliberate clippy Lint Policy" (2026-09-04)

---

## Decision: Adopt `itertools` for Iterator Composition

**Date**: 2026-09-17
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The house style mandates declarative iterator chains over imperative loops (code-quality.md, rust-blueprint §4), but five sites still needed imperative scaffolding: two validators extracted a "reference" element via `first()` + early return before comparing every element against it; the `oscillator` and `thresholds` signal-semantics modules carried manual `prev`/`curr` mutable state through a loop for pairwise crossing detection; and `metadata_report` built the (instrument x mms_id) Cartesian product with a nested for-loop. Each pattern is a solved problem in `itertools`.

### Decision
Add `itertools = "0.15"` (default features) and refactor the five sites — behavior-preserving, all existing tests kept verbatim:
1. `same_instrument_validation` + `same_indicator_types_validation`: reference-compare → `.all_equal()` over the chained iterators (empty input stays vacuously valid).
2. `oscillator` + `thresholds` signal semantics: manual prev/curr loop → `.tuple_windows()` + `filter_map` into `Either::Left`/`Either::Right` + `.partition_map()` — single pass, no mutable state.
3. `metadata_report`: nested product loop → `iproduct!` macro.

### Rationale
- Directly extends the mandated iterator-chain style; removes loop-carried mutable state from domain signal semantics
- Lazy zero-cost adaptors; `partition_map` keeps crossing detection single-pass
- Minimal dependency footprint — the only transitive dependency is `either`
- Deliberately NOT applied where a serial loop is intentional: `performance_metrics` (loop-carried cumsum/cummax), `ema` (loop-carried recursion), and rayon `par_iter` sites (itertools is sequential-only)

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Stay std-only | No new dependency | Reference-extraction boilerplate, manual prev/curr state, nested product loops remain | Those are exactly the anti-patterns the house style forbids |
| Refactor with std only (`scan`, `flat_map`) | No new dependency | Pairwise detection and cartesian products are far less readable in std | Readability is the point of the refactor |
| Apply everywhere (metrics, EMA) | Uniformity | Loop-carried state there is intentional and documented | Forced rewriting would hurt clarity, not help it |

### Impact
**Positive**: Five sites more declarative; signal semantics lose mutable state; validators lose reference-extraction boilerplate; all 429 tests pass unchanged
**Negative**: One more dependency; contributors must know the itertools API surface
**Risk**: Low — behavior-preserving refactors. Gotchas documented: `chunk_by` groups consecutive runs only (sort first for full grouping); `zip_eq` panics on length mismatch (plain `zip` is correct in `crossover`, where unequal lengths are allowed)

### Related
- Implementation: `Cargo.toml`, `src/domain/validation/same_instrument_validation.rs`, `src/domain/validation/same_indicator_types_validation.rs`, `src/domain/signal/oscillator.rs`, `src/domain/signal/thresholds.rs`, `src/application/metadata_report.rs`
- Complements: rust-blueprint §4 (Functional Patterns) and §10 (crate table — itertools added)

---

---

## Decision: Remove the `ct` Python-Wrapping CLI

**Date**: 2026-10-21
**Status**: Decided
**Owner**: Arman Mojaver

### Context
After the removal of the actix-web server scaffold and the in-memory repos (the two previous "remove dead code" sweeps), the `ct` binary was the last surface that existed solely to forward commands out of the engine. It held two forwarding paths:
1. **Engine commands** (`strategy create`, `create_loop`, `metadata`) — forwarded to `bt-engine` via `docker exec engine bt-engine …`.
2. **Data-management commands** (`dev`, `indicator`, `mms`, `operation-point`, `raw-point`, `resampled-point`, `strategy display`) — forwarded to the companion Python CLI inside the `api` container.

The `ct` binary also dragged the only Python Docker dependency into the engine, blocked compile-time improvements inside the engine container (extra source files cross-compiled for no functional gain), and added an indirection layer that confused operators about which backend handled which command. Engine commands already had a direct path (`bt-engine …`); data-management commands already had their own canonical home (the Python CLI). `ct` provided neither new capability nor a meaningful shortcut.

### Decision
Remove the `ct` binary entirely:
- `src/cli/` directory deleted (8 files: 7 command files + `cli/mod.rs`)
- `src/main.rs` deleted
- `lib.rs` no longer re-exports the `cli` module
- `Cargo.toml` `[[bin]]` entry for `ct` removed; `bte` (build) aliased for the single remaining binary

Data management and the iteration table are reached exclusively through the companion [Backtesting-4.0](../Backtesting-4.0) Python service from now on; engine commands run directly as `bt-engine …`.

### Rationale
- Deletes a forwarding layer with no unique capability — every `ct` command has a direct equivalent
- Removes the last Python/Docker dependency from the engine container, shortening compile time and shrinking the build surface
- Eliminates operator confusion about which backend handles which command
- Closes the gap between the engine and the Python service: each is one binary and one documented entry point
- Follows the same precedent as the previous "dead code" sweeps: server scaffold (2026-06-04), `strategy generate` command (2026-06-28), in-memory repos (2026-08). All were correctly diagnosed as low-value surface area

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep `ct` as a thin convenience shim | Backward compat for any operators with muscle memory | Still drags Python forward path + extra compile cost; the convenience fades | Project is single-binary; muscle memory adapts fast |
| Promote some data commands to `bt-engine` | One binary to do everything | Data CRUD belongs with persistence + APIs (Python/Django); engine stays compute-only | Wrong layer for data management |
| Replace `ct` with a new dispatcher binary | Same UX, cleaner impl | Re-introduces the indirection problem the removal solves | YAGNI; direct commands work |

### Impact
**Positive**: Single Rust binary; no Python or Docker dependency in the engine; cleaner compile surface; clearer operator mental model; one less thing to install
**Negative**: Operators must invoke `bt-engine …` directly and reach the Python service directly for data/display; the historical `ct` alias is gone
**Risk**: Documentation drift — README, context files, and the Delta Lake PRD's planned `ct strategy read`/`vacuum` commands needed (and received) follow-up cleanup to land those new commands on `bt-engine` directly

### Related
- Commit `68391e9` (chore: remove "ct" cli and improve compile time inside container)
- Implementation: deletion of `src/cli/*.rs` (8 files) + `src/main.rs` (9 deleted, 742 −11 in the commit)
- Similar removals: server scaffold (2026-06-04), `strategy generate` (2026-06-28), in-memory repos (2026-08)
- Documentation updates: `README.md`, `.opencode/context/project-intelligence/{domain-model,business-tech-bridge,business-domain,living-notes}.md`, `.opencode/.tmp/prd/2026-10-18-delta-lake-migration.md`

---

## Decision: Remove the `strategy metadata` Command

**Date**: 2026-10-25
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`bt-engine strategy metadata` was a read-only diagnostic: it cross-referenced DB state (distinct instruments, money-management ids, indicator ids) with the processed manifest and printed the done vs remaining `(instrument, mms_id, indicator_id)` triples. It was born as a free by-product of the Processed Manifest decision (2026-08-07) and changed no state. It existed as a one-consumer surface: a CLI handler, an application module with 11 tests, and three PG adapter read methods that nothing else called. In the ongoing dead-code sweep series (server scaffold, `strategy generate`, in-memory repos, `ct` CLI), the command was removed to shrink the CLI to the engine's actual job: compute (`create` / `create_loop`).

### Decision
Remove the command and everything that existed only to serve it (465 lines total):
- `src/engine/strategy/metadata.rs` (59 lines) deleted; `StrategyCommand::Metadata` variant, subcommand, parse arm, execute arm removed from `src/engine/strategy/mod.rs`
- `src/application/metadata_report.rs` (406 lines, 11 inline tests) deleted; module removed from `src/application/mod.rs`
- Three metadata-only PG adapter methods deleted from `src/infrastructure/pg_adapter.rs`: `get_distinct_mms_ids()`, `get_distinct_instruments()`, `get_all_indicator_ids()` — adapters, constructors, and trait impls stay (used by `create_loop` and `fetch_source_data`)
- Shared infrastructure untouched: `ParquetProcessedManifest` (incl. `read_processed_ids_per_cell()`), `domain::cast`, config resolution
- README + project-intelligence docs updated to match

### Rationale
- CLI surface shrinks to the two compute commands — one mental model: `bt-engine strategy create|create_loop`
- Removes a one-consumer surface: fewer modules to lint/test, three fewer adapter methods to keep schema-synced
- The manifest ledger is untouched — done/remaining remains derivable by diffing DB ids against `processed.parquet` whenever a progress report is actually needed
- Removal is purely subtractive: exhaustive grep + clippy's denied `dead_code` lint + the full test suite verify nothing dangles

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep the command as operator tooling | Progress report for free | One-consumer surface duplicating what `create_loop` already skips/logs; costs a module + tests + adapter methods | Dead-code sweep takes precedence |
| Move reporting into the companion Python service | Keeps a packaged progress report | Adds a manifest+DB read path to the service for a diagnostic | YAGNI; derive on demand instead |
| Keep `metadata_report` module, drop only the CLI | Report logic stays testable | Dead module with zero callers — exactly what the sweep removes | Contradicts the removal |

### Impact
**Positive**: Smaller CLI surface; 465 fewer lines; three fewer adapter methods; 11 fewer tests with zero coverage loss (no other code used those paths)
**Negative**: No one-command progress report — operators derive done/remaining from the manifest vs DB when needed
**Risk**: Low — subtractive change; clippy dead-code deny + test suite gate the result

### Related
- Deprecates item 3 of: Processed Manifest + Idempotent Partitions (2026-08-07) — the manifest itself (items 1–2) remains in force
- Same sweep precedent: server scaffold (2026-06-04), `strategy generate` (2026-06-28), in-memory repos (2026-08), `ct` CLI (2026-10-21)
- Implementation: deletion of `src/engine/strategy/metadata.rs` + `src/application/metadata_report.rs`; edits to `src/engine/strategy/mod.rs`, `src/application/mod.rs`, `src/infrastructure/pg_adapter.rs`
- Documentation updates: `README.md`, `.opencode/context/project-intelligence/{domain-model,business-tech-bridge,business-domain,living-notes}.md`

---

## Decision: Arm64-Only Runtime Image Tuned for Apple Silicon (M2)

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The runtime image only ever runs on the owner's MacBook Pro (M2), but CI built it on `ubuntu-latest` with no `platforms:` setting — every published `latest`/SHA image was linux/amd64, executed on the Mac through Rosetta/QEMU emulation (≈1.5–10× slower than native arm64). Separately, notes.md asked for `RUSTFLAGS="-C target-cpu=native"`; `native` is wrong in CI (it resolves to the *runner's* CPU — Neoverse on GitHub's arm64 runners, x86 on amd64) and is invalid when cross-compiling. Finally, running the runtime image with a dev-compose-style `command: ["sh", "-c", ...]` collided with its `ENTRYPOINT ["bte"]`, producing `bte sh -c ...` → `error: unrecognized subcommand 'sh'` replayed on a `restart: unless-stopped` loop.

### Decision
1. CI publishes **linux/arm64 only**, built natively on GitHub's arm64 runners (`ubuntu-24.04-arm` — free for public repos, available in private repos since 2026-01). No QEMU, no cross-toolchain. `latest` is replaced by an arm64-only image; amd64 consumers are dropped by design.
2. M2 tuning is explicit, not `native`: `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C target-cpu=apple-m2 -C link-arg=-fuse-ld=mold"` in the builder stage. Verified on rustc 1.98.1: accepted, and its 29-feature set (lse, rcpc/rcpc2, fp16, dotprod, i8mm, bf16, …) is identical to `native` on the M2 and to the listed `apple-a15` (same Avalanche microarchitecture; fallback name if a future LLVM drops the alias). Bare `cargo build --release` on the Mac may still use `target-cpu=native` — identical CPU. The per-target env form keeps host build scripts unflagged.
3. Runtime base: `debian:trixie-slim` mirroring the dev container's userland (bash, coreutils, ca-certificates) minus the toolchain — same Debian release as the builder (glibc 2.41 matches); non-root `bte` user (uid 10001, as in the original runtime image); `mkdir /app/logs /app/parquet_files` owned by `bte`; `CMD ["sleep", "infinity"]` exactly like dev; **no ENTRYPOINT** — it is a workspace container, not a batch entrypoint (the ENTRYPOINT design was what turned a compose `sh -c` command into `bte sh -c ...` → "unrecognized subcommand 'sh'"). 157 MB (measured) vs 209 MB before. A first iteration of this change shipped distroless `cc-debian13:nonroot` (65 MB, no shell) before the owner clarified the runtime workflow is get-in-and-call-the-CLI, matching the dev container — which requires the Debian userland.
4. Release profile: `panic = "abort"` added (unwind tables dropped; clippy already denies panics); `lto = "thin"`, `codegen-units = 1`, `debug = "line-tables-only"` kept. The Docker build then runs `strip --strip-debug` on the final ELF — on Linux the line tables embed in the binary (~40 MB here; macOS keeps them outside via `split-debuginfo=unpacked`), so the shipped binary drops to ~8.8 MB with the symbol table intact (backtraces keep function names; local bare builds keep full info).
5. `mimalloc` global allocator (`src/bin/bt-engine.rs`) — the hot path churns Arrow/Parquet buffers across many rayon threads.
6. Feature-trimmed `arrow`/`parquet`: ZSTD only — `snap`/`brotli`/`flate2`/`lz4` compile out of the dependency graph (the engine writes and reads ZSTD exclusively).
7. Builder dependency caching: fake-source dependency-compile layer — only `Cargo.toml`/`Cargo.lock` changes rebuild dependencies; source edits rebuild just the workspace crate. (GHA `mode=max` layer caching cannot do this once `COPY . .` invalidates the layer, and BuildKit cache mounts do not export to the GHA backend.)
8. `mold` installed in the builder — release links are the slow tail under lto + codegen-units=1.
9. Runtime workflow: `docker-compose.runtime.yaml` is a long-running workspace service (`restart: unless-stopped`, `platform: linux/arm64`, parquet + logs mounts) in its own compose project (`name: bte-runtime`, so dev and runtime containers can coexist). Makefile targets mirror the dev set with an `r` suffix: `upr`, `inr` (bash inside, like `in`), `downr`, `buildr` (renamed from `build-release`), `pushr`, `pullr`. One-shot runs: `docker compose -f docker-compose.runtime.yaml run --rm engine bte <args>`. The dev compose pins `platform: linux/arm64` as a guardrail.

### Rationale
- Native arm64 removes the emulation tax — the single largest runtime win (1.5–10×)
- Explicit `apple-m2` enables LSE atomics (rayon's work-stealing is atomics-heavy) and is safe on every build host while exact for the only deployment target
- Debian userland matches the dev container exactly, so the runtime workflow is muscle-memory identical — only the binary differs (release, M2-tuned)
- Layer-cached dependency compiles turn every-push CI docker builds into "relink the workspace crate"

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Multi-arch manifest (amd64 + arm64) | Runs anywhere | 2× build time/matrix; amd64 never used | The M2 is the only consumer |
| QEMU-emulated arm64 builds on amd64 runners | No runner change | rustc under QEMU ≈10–20× slower; LTO builds would blow the timeout | Native arm runners exist |
| Cross-compile on amd64 (zigbuild/cross) | Works without arm runners | Cross-linker maintenance | Unnecessary — arm runners cover public and private repos |
| `target-cpu=native` in CI | notes.md as written | Resolves to the runner's CPU (Neoverse/x86) — wrong ISA for the M2; invalid when cross-compiling | Explicit `apple-m2` is correct everywhere |
| Static musl + `FROM scratch` | ~10–15 MB image | musl malloc/resolver semantics change; no userland for the interactive workflow | Wrong shape for a workspace container |
| Distroless `cc-debian13:nonroot` (first iteration) | 65 MB, no shell/package surface | No `sleep`, no bash — cannot host the get-in-and-call-the-CLI workflow | Owner's workflow matches the dev container; userland required |
| `lto = "fat"` | ~1–3% runtime | ≈2× link time on 4-vCPU runners over a large dep graph | Thin LTO already set; poor CI-cost trade |
| `strip = true` in release profile | Smaller binary everywhere | Removes the symbol table too — backtraces lose function names | Builder-stage `--strip-debug` keeps names at the same size win |
| Ship the Linux binary unstripped | Full in-container backtraces | 48.7 MB binary → 114 MB image | Names-only backtraces in a ~74 MB image is the right trade here |
| UPX-compressed binary | Smallest image | Slower startup; breaks mmap page sharing | Net negative for a batch CLI |
| PGO (cargo-pgo) | ~5–15% on compute | High CI complexity; needs a representative profile workload | Deliberate follow-up, not part of this change |
| cargo-chef for dep caching | Battle-tested | Extra tool + image to pin | Fake-source layer achieves the same with zero added tools |

### Impact
**Positive**: Native-speed execution on the M2 (no emulation); 209 MB → 157 MB runtime image (−25%); shipped binary 48.7 → 8.8 MB (debug sections stripped, symbol table kept); every-push CI rebuilds only the workspace crate; M2-tuned codegen (LSE atomics et al.); faster allocation under rayon; a compose `command:` can never be misread as a `bte` subcommand (no ENTRYPOINT)
**Negative**: `latest` no longer runs on x86-64 machines (accepted — M2-only by design); an `apple-m2`-tuned binary may not run on non-Apple ARM servers (e.g. older Graviton) if ever needed — rebuild without the flag then; first CI run after this change starts a cold arm64 cache
**Risk**: non-root uid (10001) writing to macOS bind mounts — validated locally (Docker Desktop file sharing permits non-root container uids); the image ships apt, so ad-hoc tooling can be installed in a pinch

### Related
- notes.md (target-cpu advice), README.md (Docker workflow), Dockerfile, .github/workflows/ci.yml, docker-compose.runtime.yaml, docker-compose.yaml, Makefile, Cargo.toml, src/bin/bt-engine.rs, .dockerignore
- Supersedes the publishing half of: CI Workflow (2026-09-03) — same gates, new runner/arch

---

## Decision: anyhow at Domain Ports (Deviation from Blueprint §3)

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Blueprint §3 says `thiserror` for domain errors and `anyhow` only in binary entry points. §10 lists `anyhow` for application errors, so the blueprint contradicts itself. The four repository ports in the domain (`LongOperationPointRepository`, `ShortOperationPointRepository`, `ResampledPointD1Repository`, `IndicatorRepository`) return `anyhow::Result`. Batch 3 converted every domain error enum to `thiserror` and deferred the ports.

### Decision
Repository ports keep returning `anyhow::Result`. Callers add context and propagate. No caller downcasts or matches on port error variants, and no typed port error enums are introduced.

### Rationale
Each port is an infrastructure-facing seam with one adapter (PostgreSQL). A typed error enum per port adds boilerplate with no caller that branches on it. Typed port errors become worth their cost when a second adapter exists, because each adapter then needs an error vocabulary that callers can match.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Typed `thiserror` enum per port, now | Callers can match failure modes | Four enums of boilerplate; no caller matches today | Over-engineering for one adapter |
| Follow §3 literally (`thiserror` at ports, `anyhow` only in binaries) | Matches the blueprint text | Rewrites every port signature for no current consumer | Deferred until a revisit trigger fires |

### Impact
**Positive**: No port-level error ceremony for the single adapter; signatures stay small.
**Negative**: Port failures are opaque to callers. A caller that must branch has to downcast the `anyhow` error.
**Risk**: A second adapter ships without typed errors. The revisit trigger below guards against this.

**Revisit when**: a second adapter is added, a caller must branch on port errors, or the crate is published.

### Related
- Blueprint: `.opencode/context/core/standards/rust-blueprint.md` §3 (crate recommendation) and §10 (crate table)
- Code: `src/domain/operation_point.rs`, `src/domain/resampled_point.rs`, `src/domain/indicator.rs` (port traits)
- Batch 3 PRD: `.opencode/.tmp/prd/2026-11-01-batch-3-structural-quality-typed-errors-tests.md` (typed port errors out of scope)

---

## Decision: Total-Loss Annualisation Guard (NaN Becomes 0.0)

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
An extreme loss sequence can push the accumulated value `1 + global_roi/100` to zero or below. `powf` on a negative base yields NaN. The persistence path converts `annual_roi * 100` with `round_f64_to_i128`, which maps NaN to 0. A total loss was therefore persisted as `0.00`, indistinguishable from a real zero.

### Decision
`PerformanceMetrics::annual_roi` returns `0.0` when the accumulated value is `<= 0`, before `powf`. This matches the existing degenerate-input convention: zero and negative spans already return `0.0`.

### Rationale
`0.0` is finite and sits below the 5 % ROI bar, so the quality stage drops the strategy. No NaN-derived value reaches the Parquet output.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Leave `powf` unguarded | No code change | NaN is persisted as `0.00` | Silent corruption of the output |
| Fail the whole cell with an error | Makes the case visible | One pathological strategy aborts a batch | A total loss is a valid outcome, not a fault |
| Return -100 % | Finite, and also dropped by the bar | A second sentinel next to the existing `0.0` convention | Inconsistent with the zero-span convention |

### Impact
**Positive**: `annual_roi` is always finite, and the behaviour is pinned by tests.
**Negative**: A total-loss strategy reads as 0 % annual ROI, not -100 %.
**Risk**: Low. The quality bar removes these strategies before output.

### Related
- Commit `84176b7` (guard and tests); `src/domain/performance_metrics.rs`
- Batch 1 PRD: `.opencode/.tmp/prd/2026-11-01-batch-1-robustness-code-touchups.md`
- Same batch: the import removal and per-cell borrow (commit `cd95311`) are mechanical and not recorded separately.
- Not landed: the debug-only assert on the usize cast helper (PRD 1). Pending.

---

## Decision: Typed Domain Errors via thiserror

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Seven error enums hand-rolled `Display` and `std::error::Error`, about 200 lines of mechanical match arms. Blueprint §3 names `thiserror` for exactly this case. The three per-invariant validation enums did not implement `Display` at all.

### Decision
Adopt `thiserror` with no features. All seven enums derive `Error` with `#[error(...)]` attributes. Every message is byte-identical to the previous text, and the three per-invariant enums gain a `Display` that mirrors the combined error's phrasing.

### Rationale
Message text lives once, at the variant. The existing message test tables stay verbatim as the proof that output did not change. The dependency is in the house style the blueprint already mandates.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep hand-rolled impls | No new dependency | Repetitive and error-prone; the blueprint's named anti-pattern | Mechanical boilerplate |
| `anyhow` for domain errors | Simple propagation | Callers cannot match on variants | Domain errors must stay typed |

### Impact
**Positive**: The conversion commit removed 175 lines and added 81. Message text is defined once.
**Negative**: One more proc-macro dependency (`thiserror` 2).
**Risk**: Low. Message byte-equality is pinned by tests.

### Related
- Commit `32e913b`; `Cargo.toml` (`thiserror = "2"`)
- Precedent: itertools adoption (2026-09-17, above)
- Blueprint §3 (`thiserror` for domain errors)

---

## Decision: Pipeline Composition Tests over Existing Seams

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The use-case orchestrator (`run_strategies`) and the per-strategy builder (`create_strategy`) compose the whole pipeline. Every callee had tests, but the composition contract was unproven: validation runs, both filter stages apply, and signals resolve to trades.

### Decision
Test the composition through the seams that already exist. The orchestrator is tested over shared domain fixtures. The builder is tested as a pure function over hand-built timestamp maps, with hand-computed metric expectations. No new seams.

### Rationale
The highest existing seam covers the contract, so a dropped stage fails a test. New seams would add surface area without adding a capability.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| New trait-based test doubles | Fine-grained control | A new seam with no need | Existing seams already reach the contract |
| Database-backed integration tests | Exercises real queries | Needs PostgreSQL; the suite runs without one | Breaks the no-database test policy |

### Impact
**Positive**: Dropping a filter stage or an unresolved-signal rule now fails a test.
**Negative**: Fixtures must stay representative as the domain evolves.
**Risk**: Low.

### Related
- Commit `0c82ee1`; `src/application/strategy_runner.rs`, `src/application/strategy_generator.rs`
- Batch 3 PRD: `.opencode/.tmp/prd/2026-11-01-batch-3-structural-quality-typed-errors-tests.md`

---

## Decision: CI Triggers and Preflight Gate (Pending)

**Date**: 2026-11-01
**Status**: Pending
**Owner**: Arman Mojaver

### Context
Lint and test sit behind a Docker Hub preflight login. An expired token gives zero code-quality feedback. The workflow runs only on pushes to `main`, so branches get no feedback before merge.

### Decision
Accepted, not yet implemented: remove the preflight job (the docker job already logs in on its own), add `pull_request` to the triggers, and gate image publishing to `main`. `deny` runs on pull requests because it needs no secrets. As of 2026-11-01, `.github/workflows/ci.yml` is unchanged: the preflight gate and push-only trigger are still in place.

### Rationale
Code-quality feedback should not depend on an unrelated credential, and branches should be gated before merge. Publishing stays exclusive to `main`. Accepted trade-off: a revoked token costs one lint and test cycle before failing at the docker login.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep the preflight gate | No workflow change | Lint and test are skipped entirely on a credential failure | Hides the quality signal it was meant to protect |
| Keep push-only triggers | No workflow change | No gate before merge | Branch work gets no feedback |

### Impact
**Positive** (once implemented): Feedback on every branch and pull request; images still publish only from `main`.
**Negative**: One wasted lint and test cycle on a revoked token.
**Risk**: A wrong job condition could publish from a pull request. The publish-only-on-`main` condition must be verified when implemented.

### Related
- Batch 2 PRD: `.opencode/.tmp/prd/2026-11-01-batch-2-ci-tooling-hardening.md`
- Current workflow: `.github/workflows/ci.yml` (its header DAG comment describes the current preflight gate)
- Earlier decision: CI Workflow (2026-09-03)

---

## Decision: Local Make Targets Mirror CI

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
The Makefile `test` target omitted `--locked`, and the `ci` target ran its gates in a different order from the workflow.

### Decision
`make test` runs `cargo nextest run --locked`. `make ci` runs lint, then test, then deny, matching the pipeline it mirrors.

### Rationale
"Passes locally" should mean "passes CI", and the lockfile must be enforced everywhere.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Leave the Makefile independent | No churn | Drift between local and CI results | Defeats the purpose of `make ci` |

### Impact
**Positive**: Local lint, test, and deny gates match CI.
**Negative**: None measurable.
**Risk**: Drift remains. The rustdoc gate that CI runs (`cargo doc` with `-D warnings`) is not in `make lint` or `make ci`.

### Related
- Commit `187cc46`; `Makefile`
- Batch 2 PRD: `.opencode/.tmp/prd/2026-11-01-batch-2-ci-tooling-hardening.md`

---

## Decision: Keep criterion as a Dev-Dependency

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`criterion` is a dev-dependency with no in-tree benches: `autobenches` is off and `benches/` holds only a placeholder. It exists for ad-hoc `/bench` measurement sessions. Nothing in the repository said so, so it read as dead configuration.

### Decision
Keep `criterion` (with `html_reports`) and document its purpose in `Cargo.toml`, at the declaration.

### Rationale
`/bench` expects the dependency to be present. The dev-build compile cost is the accepted price of that workflow.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Remove it, and re-add on demand | Faster dev compiles | `/bench` breaks until it is re-added | `/bench` expects it present |

### Impact
**Positive**: Reviewers see the intent at the declaration.
**Negative**: The dev-only compile cost stays.
**Risk**: None.

### Related
- Commit `927da0c`; `Cargo.toml` (`[dev-dependencies]`); `benches/` (placeholder only)
- Batch 2 PRD: `.opencode/.tmp/prd/2026-11-01-batch-2-ci-tooling-hardening.md`

---

## Decision: One PgStore Serves All Four Repository Ports

**Date**: 2026-11-01
**Status**: Decided (trait merge pending)
**Owner**: Arman Mojaver

### Context
Four PostgreSQL adapter structs duplicated connection boilerplate. The single-cell `create` path opened four connections for four sequential reads. The domain also declares two operation-point traits with identical signatures.

### Decision
One `PgStore` holds a single connection and implements all four repository ports. `create` constructs it once. `create_loop` keeps four `PgStore` instances, one per table, because its loads run concurrently and `postgres::Client` is single-connection. A comment in `create_loop.rs` states why.

**Pending:** merging the two operation-point traits (`LongOperationPointRepository`, `ShortOperationPointRepository`). Batch 4 PRD user story 2 asks for it, but its implementation decisions do not include it. Not executed.

### Rationale
Sequential work needs one connection. Concurrent work keeps its own connections.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| One connection for the batch path too | Fewer connections | The four loads would serialize | Loads run concurrently |
| Keep four adapter structs | No restructuring | Duplicated boilerplate; four connections for single-cell | The copy-paste the PRD targets |

### Impact
**Positive**: Less boilerplate; single-cell `create` uses one connection.
**Negative**: `PgStore` implements all four ports, so it is a larger type.
**Risk**: Adapters have no database-backed tests by design; the change was verified by review.

### Related
- Commit `b4d40ac`; `src/infrastructure/pg_adapter.rs` (`PgStore`), `src/application/source_data.rs`, `src/engine/strategy/create_loop.rs`
- Batch 4 PRD: `.opencode/.tmp/prd/2026-11-01-batch-4-optional-refinements-exit-date-metrics.md`

---

## Decision: PathBuf for Filesystem Paths; Batch 4 Menu Deferrals

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
Batch 4's optional menu (PRD 4) offered four refinements: a PathBuf migration (only alongside consolidation), a per-cell indicator clone, dispatch-test deduplication, and edition 2024.

### Decision
Ship the PathBuf migration across the environment, manifest, and adapter paths. Defer the other three:
- Edition 2024: stay on edition 2021 under the exact-pin policy. Deliberate conservatism.
- Per-cell indicator clone: measure before changing.
- Dispatch-test deduplication: light touch only, when the test builders next change.

### Rationale
`PathBuf` removes string-based path handling without changing behaviour. The remaining items are either conservatism under the toolchain pin or changes that need measurement first.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Edition 2024 now | Newer language defaults | Moves the pinned toolchain's rules with no current need | Deferred under the exact-pin policy |
| Clone avoidance now | Less allocation | Unmeasured benefit | Measure first |

### Impact
**Positive**: Typed paths in the migrated files.
**Negative**: The edition stays on 2021.
**Risk**: Low.

### Related
- Commit `a4bc3b9`; `Cargo.toml` (`edition = "2021"`)
- Batch 4 PRD: `.opencode/.tmp/prd/2026-11-01-batch-4-optional-refinements-exit-date-metrics.md`

---

## Decision: Exit-Date Metrics Requirements Recorded, Implementation Deferred

**Date**: 2026-11-01
**Status**: Pending
**Owner**: Arman Mojaver

### Context
ROI and max draw down compound trades in entry order. Real trades close at exit dates that are not in entry order, so the figures are approximate. The operation-point table's schema is owned by the companion Python service, so the fix needs cross-repository coordination.

### Decision
Record the requirements now, and implement them under a follow-up PRD after the companion schema is coordinated. Requirements: an exit-date column owned by the companion; the adapter reads it; `OperationPoint` gains the field; the compounding walk orders by exit; fixtures prove that entry-order and exit-order results differ. Open semantics for the follow-up: overlapping exposure, and whether the annual operation count counts per exit year.

### Rationale
Changing the adapter before the schema is coordinated risks a breaking window. The Parquet output schema is unaffected, so only the metric values change.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Implement now | Removes the approximation | Schema not coordinated; breaking-window risk | Coordinate first |
| Accept entry-order approximations indefinitely | No work | Blocks production use of ROI and draw down | Living notes mark this as required before production use |

### Impact
**Positive**: Requirements are on record, and the limitation stays visible in the README and living notes.
**Negative**: Metrics stay approximate until the follow-up ships.
**Risk**: ROI and draw-down figures used for production decisions before the follow-up.

### Related
- Batch 4 PRD: `.opencode/.tmp/prd/2026-11-01-batch-4-optional-refinements-exit-date-metrics.md`
- Living notes: Technical Debt (exit-date ordering), Active Projects (exit-date-aware metrics)
- README: Known limitations, item 1

---

## Decision: Retain the Unused `tp` Field as the Exit-Date Anchor

**Date**: 2026-11-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`OperationPoint.tp` is loaded from the source table, but no computation reads it. Only the adapter and test fixtures touch it.

### Decision
Keep the field and record its purpose as the reserved anchor for exit-date modeling (see the domain model). No computation reads it today.

### Rationale
Exit-date modeling needs a take-profit reference. Removing the column now and re-adding it later would cross the same schema coordination the exit-date work needs anyway.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Remove the field now | Less unused data in the query | Re-adding later means another schema change | Exit-date work needs the schema change anyway |
| Use `tp` in a computation now | Gives the field a reader | No current computation needs it | Not in scope |

### Impact
**Positive**: The field's purpose is on record here and in the domain model.
**Negative**: A column is loaded and unused.
**Risk**: Low.

### Related
- `src/domain/operation_point.rs` (the `tp` field); `src/infrastructure/pg_adapter.rs` (the query selects `tp`); `domain-model.md` (glossary: Take Profit)
- Batch 5 PRD: `.opencode/.tmp/prd/2026-11-01-batch-5-documentation-truth-alignment.md`

---

## Decision: Remove the Same-Indicator-Type Invariant

**Date**: 2026-10-12 (removed in `917e4eb`; rationale recorded 2026-11-02)
**Status**: Decided
**Owner**: Arman Mojaver

### Context
`validate_same_indicator_types` rejected any run whose indicators did not share one type. `run_strategies` validates the full indicator list the database returns, so a database holding two types failed every run.

### Decision
Remove the validator and its call, so one run may mix indicator types. The `type` field had already left the domain struct (`27b7fe7`, 2026-10-23); the type now lives only in the database.

### Rationale
Indicators used to run together by type. `create_loop` now runs different types together, and the processed manifest records which indicators are unprocessed regardless of type, so the per-type grouping no longer serves any purpose.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep the check | Each run is type-homogeneous | Any database holding two types fails every run | Mixed-type runs are now the intended mode |

### Impact
**Positive**: A database can hold any mix of indicator types, and idempotency stays per indicator.
**Negative**: No type-level guard remains on a run.
**Risk**: Low. The other validators in `src/domain/validation/` still run.

### Related
- Commits `917e4eb` (removal) and `27b7fe7` (type removed from the domain struct)
- `src/application/strategy_runner.rs` (`run_strategies`); `src/engine/strategy/create_loop.rs`
- Deprecated Decisions table: "Same-indicator-type validation"

---

## Decision: MACD Is a Crossover of the MACD Line and Its Signal Line

**Date**: 2026-11-02
**Status**: Decided (implemented in Batch 6)
**Owner**: Arman Mojaver

### Context
MACD landed in `8c5f4e2` (2026-10-12) as an Oscillator: a zero-cross of the MACD line. The owner's intent is that MACD is a crossover. The `Crossover` signal kind had no producer. The database stores MACD parameters as `slow_type`, `slow_n`, `fast_type`, and `fast_n`; the companion schema rejects extra keys, so no signal period is stored.

### Decision
`macd::compute` returns a crossover pair. The long buffer is the MACD line; the short buffer is the signal line, `SMA(MACD, 9)`. Both buffers start at the first bar where the signal line exists. The signal period is an engine constant, 9: the MQL5 reference default (`Indicators/Examples/MACD.mq5`, `InpSignalSMA = 9`).

### Rationale
The crossover is the owner's intended semantics. The period comes from the reference source, and the database has no field that could carry a different one. Storing one would need a cross-repository schema change and a migration of existing rows.

### Alternatives Considered
| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Keep the Oscillator (zero-cross of the MACD line) | Already shipped and tested | Not the owner's intended semantics | Owner direction: MACD is a crossover |
| Add `signal_n` to the companion schema and the Rust adapter | Signal period configurable per row | Cross-repository schema change and migration | Deferred; the constant covers the reference default |

### Impact
**Positive**: MACD signals follow the classic line-crossing definition and match the reference source's two buffers.
**Negative**: The signal period cannot vary per row. Existing MACD-based strategies change; the Parquet schema does not.
**Risk**: With `ema`/`ema` lines at 12 and 26, MACD's signals are exactly the zero-crosses of OsMA(12, 26, 9). The two indicators agree in that configuration rather than adding independent information.

### Related
- `src/domain/indicators/macd.rs` (`compute`, `SIGNAL_N`); `src/domain/indicators/mod.rs` (dispatch arm)
- `src/domain/indicators/osma.rs` (same signal-line alignment)
- Companion repository: `schemas/indicator/macd_schema.py` (`MacdParametersSchema`)
- Batch 6 PRD: `.opencode/.tmp/prd/2026-11-01-batch-6-macd-crossover.md`

---

## Deprecated Decisions

| Decision | Date | Replaced By | Why |
|----------|------|-------------|-----|
| Redis-Backed Signal Groups Cache | 2026-06-08 | Remove Redis (2026-08-19) | Superseded by in-memory `cached` memoization |
| Separate `global_roi`/`max_draw_down`/`annual_roi` metric files | 2026-06-24/28 | Single-pass `performance_metrics()` (2026-08-31) | All three derive from one compounding curve; combined into one pass with a defensive sort owned by the function |
| Random-timestamp signal generation | pre-2026-07 | Port Indicators to Rust + Real Signal Generation (2026-07) | Signals must derive from indicator mathematics, not sampling |
| In-memory repositories for tests | 2026-06 | Remove In-Memory Repositories (2026-08-29) | Fixtures + tempfile-based adapter tests cover the same needs |
| `eprintln!` progress output | pre-2026-08 | Structured Logging via `log` + `simplelog` (2026-08-29) | Batch tooling needs levels, timestamps, and file routing |
| Manual Docker build/push (Makefile only) | pre-2026-09 | CI Workflow (2026-09-03) | Publishing must be gated on fmt + clippy + tests |
| `ct` Python-wrapping CLI binary | 2026-10-21 | Remove the `ct` Python-Wrapping CLI (2026-10-21) | Operator frontend had no unique capability; dragged Python forward path and Docker indirection into the engine |
| `strategy metadata` command (item 3 of the Processed Manifest decision) | 2026-08-07 | Remove the `strategy metadata` Command (2026-10-25) | Read-only diagnostic with no other consumers; done/remaining stays derivable from the manifest ledger |
| amd64 CI images (`ubuntu-latest`, no platform pin) | 2026-09-03 | Arm64-Only Runtime Image Tuned for Apple Silicon (2026-11-01) | Images run exclusively on Apple Silicon; native arm64 builds replace emulated amd64 |
| MACD skipped, not aborted (item 4 of "Port Indicators to Rust + Real Signal Generation") | 2026-07-26 | MACD port (`8c5f4e2`, 2026-10-12) and MACD Is a Crossover (2026-11-02) | Porting MACD removed the `UnsupportedIndicator` path; MACD is now a crossover of its line and signal line |
| Same-indicator-type validation (`validate_same_indicator_types`, from "Adopt `itertools`") | pre-2026-09-17 | Remove the Same-Indicator-Type Invariant (2026-10-12; recorded 2026-11-02) | Mixed-type runs are intended: `create_loop` runs types together and the processed manifest tracks each indicator regardless of type |

## Onboarding Checklist

- [ ] Understand the philosophy behind major architectural choices
- [ ] Know why certain technologies were chosen over alternatives
- [ ] Understand trade-offs that were made
- [ ] Know where to find decision context when questions arise
- [ ] Understand what decisions are pending and why

## Related Files

- `technical-domain.md` - Technical implementation affected by these decisions
- `business-tech-bridge.md` - How decisions connect business and technical
- `living-notes.md` - Current open questions that may become decisions
