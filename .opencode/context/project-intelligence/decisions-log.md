<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.6 | Updated: 2026-08-31 -->

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

## Deprecated Decisions

| Decision | Date | Replaced By | Why |
|----------|------|-------------|-----|
| Redis-Backed Signal Groups Cache | 2026-06-08 | Remove Redis (2026-08-19) | Superseded by in-memory `cached` memoization |
| Separate `global_roi`/`max_draw_down`/`annual_roi` metric files | 2026-06-24/28 | Single-pass `performance_metrics()` (2026-08-31) | All three derive from one compounding curve; combined into one pass with a defensive sort owned by the function |

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
