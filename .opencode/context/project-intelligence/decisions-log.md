<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.3 | Updated: 2026-06-26 -->

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
**Status**: Decided
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


## Deprecated Decisions

| Decision | Date | Replaced By | Why |
|----------|------|-------------|-----|
(none yet)

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
