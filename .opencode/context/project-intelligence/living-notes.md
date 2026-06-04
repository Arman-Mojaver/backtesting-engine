<!-- Context: project-intelligence/notes | Priority: high | Version: 1.1 | Updated: 2026-06-04 -->

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

### Technical Debt Details

**Missing HTTP API layer**
*Priority*: Low
*Impact*: No REST/HTTP interface exists for the backtesting engine. CLI-only.
*Root Cause*: Actix-web scaffold was removed as dead code in commit 1f646c9. Deliberate -- no API consumers yet.
*Proposed Solution*: Add API layer when a use case emerges (web UI, external service integration).
*Effort*: Large
*Status*: Deferred

## Open Questions

| Question | Stakeholders | Status | Next Action |
|----------|--------------|--------|-------------|
| Should engine commands be extended with strategy delete/update? | Arman Mojaver | Open | Add commands as needed |
| Should the bt binary forward engine commands to bt-engine? | Arman Mojaver | Open | Currently separate binaries |

### Open Question Details

**Engine command scope**
*Context*: Currently `bt-engine strategy` only has `create`. The `bt strategy` CLI group (Python) has more commands (delete, display_table, terminate, filters). Should these be duplicated in the Rust engine?
*Stakeholders*: Arman Mojaver
*Options*:
- Option A: Add all strategy commands to `bt-engine` (Rust-native)
- Option B: Keep `bt` as the frontend that dispatches to either Python CLI or `bt-engine`
- Option C: Keep them separate
*Timeline*: No deadline
*Status*: Open

## Known Issues

| Issue | Severity | Workaround | Status |
|-------|----------|------------|--------|
| No tests for engine module | Medium | Manual verification | Known |

### Issue Details

**Engine module has no tests**
*Severity*: Medium
*Impact*: `bt-engine` CLI commands (strategy create) are untested. Regression risk when adding new engine commands.
*Reproduction*: N/A -- missing test coverage.
*Root Cause*: Engine module is newly extracted (commits 1f646c9, b3930c6). Tests not yet written.
*Fix Plan*: Add unit tests for `engine::strategy::StrategyCommand::execute()` and integration tests for `engine::build_cli()` + `engine::parse()`.
*Status*: Known

## Insights & Lessons Learned

### What Works Well
- **`build -> parse -> execute` pattern** -- Consistent across both binaries. Simple, predictable, easy to test.
- **Pure validators as free functions** -- No object construction needed. Compose well in `combined_validation::validate_all()`.
- **Separate engine binary** -- Clear line between Python-wrapping CLI (`bt`) and native Rust engine (`bt-engine`).

### What Could Be Better
- **Test coverage** -- No tests for the engine module yet. Should have at minimum unit tests for `StrategyCommand::execute()`.
- **Context files lagging** -- The `.opencode/context/` project intelligence files were outdated after the major restructure. This update addresses that.

### Lessons Learned
- **Keep context files in sync with code** -- After major refactors, update `.opencode/context/` immediately. Outdated context misleads AI agents.
- **Delete dead code early** -- Removing the server scaffold saved ~1700 lines from Cargo.lock and eliminated misleading references to Actix-web dependencies.

## Patterns & Conventions

### Code Patterns Worth Preserving
- **`build -> parse -> execute`** -- Every binary follows this 3-step pattern. Lives in `cli/mod.rs` and `engine/mod.rs`. Consistent, predictable.
- **Pure validation functions** -- `src/domain/validation/*.rs`. Each is a free function taking slices, returning `Result`. Composable.
- **Fixture module** -- `domain::fixtures.rs` provides reusable test data across all test modules. Single source of truth.

### Gotchas for Maintainers
- **Two binaries to install** -- After changes to `engine/`, run `cargo install --path . --bin bt-engine --force`. After changes to `cli/`, run `cargo install --path . --bin bt --force`. Or use `--all` to install both.
- **Python CLI dependency** -- `bt` commands (dev, indicator, etc.) forward to a Python Docker container. The `bt-engine` binary does NOT depend on Docker.

## Active Projects

| Project | Goal | Owner | Timeline |
|---------|------|-------|----------|
| Engine CLI extraction | Move Rust engine commands into standalone `bt-engine` binary | Arman Mojaver | Complete (2026-06-04) |
| Context file update | Update .opencode/context/ to reflect current architecture | Arman Mojaver | In progress (this task) |

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
