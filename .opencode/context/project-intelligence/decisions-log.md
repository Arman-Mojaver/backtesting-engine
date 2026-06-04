<!-- Context: project-intelligence/decisions | Priority: high | Version: 1.1 | Updated: 2026-06-04 -->

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
| Keep engine as subcommand of `cc` | Single binary, minimal change | Mixed Python/Rust routing, confusing `docker exec engine` indirection | Makes CLI architecture unclear |
| Single `cc` binary with internal dispatch | Single entry point | Over-complicated dispatch logic, tight coupling | Engine and CLI have different deployment needs |

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

### Context
The `Cargo.toml` originally defined a single `[[bin]]` entry for the `cc` binary. After extracting the engine commands, we had two separate command trees that needed their own entry points.

### Decision
Define two `[[bin]]` entries in `Cargo.toml`:
- `cc` at `src/main.rs` — wraps the Python CLI (`cli/`)
- `bt-engine` at `src/bin/bt-engine.rs` — native Rust engine (`engine/`)

### Rationale
- Each binary has a focused responsibility
- Both share the same library crate (`backtesting-engine`) for shared types
- Users can install just what they need: `cargo install --path . --bin bt` or `cargo install --path . --bin bt-engine`
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
