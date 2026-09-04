<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.9 | Updated: 2026-09-04 -->

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

## Deprecated Decisions

| Decision | Date | Replaced By | Why |
|----------|------|-------------|-----|
| Redis-Backed Signal Groups Cache | 2026-06-08 | Remove Redis (2026-08-19) | Superseded by in-memory `cached` memoization |
| Separate `global_roi`/`max_draw_down`/`annual_roi` metric files | 2026-06-24/28 | Single-pass `performance_metrics()` (2026-08-31) | All three derive from one compounding curve; combined into one pass with a defensive sort owned by the function |
| Random-timestamp signal generation | pre-2026-07 | Port Indicators to Rust + Real Signal Generation (2026-07) | Signals must derive from indicator mathematics, not sampling |
| In-memory repositories for tests | 2026-06 | Remove In-Memory Repositories (2026-08-29) | Fixtures + tempfile-based adapter tests cover the same needs |
| `eprintln!` progress output | pre-2026-08 | Structured Logging via `log` + `simplelog` (2026-08-29) | Batch tooling needs levels, timestamps, and file routing |
| Manual Docker build/push (Makefile only) | pre-2026-09 | CI Workflow (2026-09-03) | Publishing must be gated on fmt + clippy + tests |

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
