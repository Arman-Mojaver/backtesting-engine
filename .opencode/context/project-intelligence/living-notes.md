<!-- Context: project-intelligence/notes | Priority: high | Version: 1.2 | Updated: 2026-06-08 -->

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
| Engine module has no unit tests | Regression risk for engine CLI commands | Medium | Add tests for StrategyCommand::execute() and RedisCommand::execute() |

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
*Impact*: `bt-engine` CLI commands (strategy create, redis check/clean) are untested at the integration level. Regression risk when adding new engine commands.
*Root Cause*: Engine module is newly extracted (commits 1f646c9, b3930c6, 2026-06-08). Tests not yet written.
*Proposed Solution*: Add integration tests that exercise engine commands against a real Redis instance (in CI) or use mocks for the Redis connection.
*Effort*: Medium
*Status*: Known

## Open Questions

| Question | Stakeholders | Status | Next Action |
|----------|--------------|--------|-------------|
| Should engine commands be extended with strategy delete/update? | Arman Mojaver | Open | Add commands as needed |
| Should the ct binary forward engine commands to bt-engine? | Arman Mojaver | Open | Currently separate binaries |
| Should the engine's compute path check Redis cache before computing signal groups? | Arman Mojaver | Open | Follow-up after cache infrastructure is complete |

### Open Question Details

**Engine command scope**
*Context*: Currently `bt-engine strategy` only has `create`. The `bt strategy` CLI group (Python) has more commands (delete, display_table, terminate, filters). Should these be duplicated in the Rust engine?
*Stakeholders*: Arman Mojaver
*Options*:
- Option A: Add all strategy commands to `bt-engine` (Rust-native)
- Option B: Keep `ct` as the frontend that dispatches to either Python `bt` CLI or `bt-engine`
- Option C: Keep them separate
*Timeline*: No deadline
*Status*: Open

**Cache integration in compute path**
*Context*: The Redis cache infrastructure is built and the CLI can read/write/delete signal groups. However, the actual engine computation path does not yet check the cache before computing, nor does it write results to the cache after compute. This is a separate follow-up.
*Stakeholders*: Arman Mojaver
*Options*:
- Option A: Integrate cache check at the start of signal group computation
- Option B: Keep cache as purely manual (operator uses ct redis before/after)
*Timeline*: Next iteration
*Status*: Open

## Known Issues

| Issue | Severity | Workaround | Status |
|-------|----------|------------|--------|
| No integration tests for engine module | Medium | Manual verification | Known |
| Redis connection hard-fails if container is down | Low | Start Redis container first | By design |

### Issue Details

**Engine module has no integration tests**
*Severity*: Medium
*Impact*: `bt-engine` CLI commands (strategy create, redis check/clean) are untested at the integration level.
*Reproduction*: N/A -- missing test coverage.
*Root Cause*: Engine module is newly extracted. Tests not yet written.
*Fix Plan*: Add integration tests that spin up a Redis instance (or use a mock) and exercise the engine CLI flow.
*Status*: Known

**Redis connection hard-fail**
*Severity*: Low
*Impact*: If the Redis container is not running, `ct redis` / `bt-engine redis` will immediately error.
*Root Cause*: By design — hard fail on connection error ensures cache problems are surfaced immediately rather than silently ignored.
*Status*: By design (documented in decisions-log.md)

## Insights & Lessons Learned

### What Works Well
- **`build -> parse -> execute` pattern** -- Consistent across both binaries. Simple, predictable, easy to test.
- **Pure validators as free functions** -- No object construction needed. Compose well in `combined_validation::validate_all()`.
- **Separate engine binary** -- Clear line between Python-wrapping CLI (`ct`) and native Rust engine (`bt-engine`).
- **TypedKey pattern** -- Compile-time type safety for Redis keys prevents serialization drift. Zero runtime overhead.

### What Could Be Better
- **Test coverage** -- No integration tests for the engine module yet. Should have tests for both `StrategyCommand::execute()` and `RedisCommand::execute()`.
- **Cache integration** -- The Redis cache infrastructure exists but is not yet integrated into the engine's compute path. Requires a follow-up.

### Lessons Learned
- **Keep context files in sync with code** -- After major refactors, update `.opencode/context/` immediately. Outdated context misleads AI agents.
- **Delete dead code early** -- Removing the server scaffold saved ~1700 lines from Cargo.lock and eliminated misleading references to Actix-web dependencies.
- **Simple key constants** -- Using a plain `&str` key constant kept a class of runtime bugs (wrong type written to wrong key) at zero performance cost.

## Patterns & Conventions

### Code Patterns Worth Preserving
- **`build -> parse -> execute`** -- Every binary follows this 3-step pattern. Lives in `cli/mod.rs` and `engine/mod.rs`. Consistent, predictable.
- **Pure validation functions** -- `src/domain/validation/*.rs`. Each is a free function taking slices, returning `Result`. Composable.
- **Fixture module** -- `domain::fixtures.rs` provides reusable test data across all test modules. Single source of truth.
- **`TypedKey<T>` pattern** -- Simple `&str` key constant with no generic overhead. Implemented in `src/infrastructure/redis.rs`.

### Gotchas for Maintainers
- **Two binaries to install** -- After changes to `engine/`, run `cargo install --path . --bin bt-engine --force`. After changes to `cli/`, run `cargo install --path . --bin ct --force`. Or use `--all` to install both.
- **Python CLI dependency** -- `ct` commands (dev, indicator, etc.) forward to a Python Docker container. The `bt-engine` binary does NOT depend on Docker.
- **Redis must be running** -- Before using `ct redis` or `bt-engine redis`, ensure the Redis container is up: `docker compose up -d redis`.
- **Redis is hardcoded** -- The Redis connection string `redis://redis:6379/0` is hardcoded, not configurable via environment. Works via Docker Compose network.

## Active Projects

| Project | Goal | Owner | Timeline |
|---------|------|-------|----------|
| Engine CLI extraction | Move Rust engine commands into standalone `bt-engine` binary | Arman Mojaver | Complete (2026-06-04) |
| Redis signal groups cache | Add Redis-backed ephemeral cache for signal group computation | Arman Mojaver | Complete (2026-06-08) |
| Cache integration in compute path | Wire engine's compute logic to check Redis before computing | Arman Mojaver | Next iteration |

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

### Resolved: Redis signal groups cache
- **Resolved**: 2026-06-08
- **Resolution**: Redis container added to Docker Compose, `src/infrastructure/redis.rs` with typed cache, `bt-engine redis` and `ct redis` CLI commands. Cache infrastructure complete — compute path integration is a follow-up.
- **Learnings**: Simple key constants prevent serialization bugs. Hard fail on connection errors is the right choice for this use case.

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
- `technical-domain.md` - Technical context for current state
- `business-tech-bridge.md` - Context for current trade-offs
