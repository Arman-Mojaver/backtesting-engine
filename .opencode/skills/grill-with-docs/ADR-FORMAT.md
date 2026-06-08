# Decision Entry Format (for `decisions-log.md`)

A decision entry in `.opencode/context/project-intelligence/decisions-log.md` captures a hard-to-reverse architectural decision with its context and trade-offs. Only write one when all three hold: the decision is hard to reverse, surprising without context, and the result of a real trade-off.

Append new entries to the bottom of `decisions-log.md`, above the "Deprecated Decisions" section.

---

## Format

```markdown
## [Decision Title]

**Date**: YYYY-MM-DD
**Status**: Decided | Pending | Under Review | Deprecated
**Owner**: [Who owns this decision]

### Context

[What situation or problem forced this decision?
What constraints existed? What were we trying to achieve?]

### Decision

[What did we decide? State it clearly and directly.]

### Rationale

[Why this decision? What made it the right choice?]

### Alternatives Considered

| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| [Option A] | [Pros] | [Cons] | [Why not chosen] |
| [Option B] | [Pros] | [Cons] | [Why not chosen] |

### Impact

**Good**: [What this enables or improves]
**Bad / Trade-offs**: [What this costs or constrains]
**Risk**: [What could go wrong]

### Related

- [Links to related decisions, PRs, issues, or documentation]
```

---

## Worked Example

The following is a real decision entry from this project's `decisions-log.md`:

```markdown
## Decision: Separate Long/Short Repositories Instead of Unified OperationPointRepository

**Date**: 2025-06-01
**Status**: Decided
**Owner**: Arman Mojaver

### Context

The backtesting engine operates on two distinct sets of trade operations:
long positions and short positions. Both are represented by the same
`OperationPoint` struct, but they originate from separate database tables
(`long_operation_point` and `short_operation_point`).

Initial designs considered a single `OperationPointRepository` with a
direction filter parameter. However, the engine's validation pipeline
(`CombinedValidation`) already treats long and short operations as separate
inputs that must be independently validated (empty checks, MMS alignment,
timestamp matching).

We needed to decide whether to use one repository or two.

### Decision

Define two separate repository traits in the domain layer:

- `LongOperationPointRepository` — with methods `get_all()` and
  `get_by_mms_id_and_instrument()`
- `ShortOperationPointRepository` — with the same method signatures

Each has its own PostgreSQL adapter (`PgLongOperationPointAdapter`,
`PgShortOperationPointAdapter`) and its own in-memory test double
(`InMemoryLongRepo`, `InMemoryShortRepo`).

### Rationale

The validation pipeline needs independent access to both sets of operations.
A unified repo would either query both tables every time or introduce an
awkward direction parameter that leaks infrastructure concerns into callers.

### Alternatives Considered

| Alternative | Pros | Cons | Why Rejected? |
|-------------|------|------|---------------|
| Single `OperationPointRepository` with an enum direction filter | One trait to maintain | Leaks infrastructure into callers; validation pipeline loses clear separation | Creates awkward coupling |
| Keep one Postgres adapter with SQL-level branching | Single adapter class | `RefCell<Client>` connection management makes branching messy; unclear contract | Adapter's responsibility becomes ambiguous |
| Merge DB tables into one with a `direction` column | One table to query | Schema churn with no domain benefit — longs and shorts are never mixed in a single computation | No corresponding domain reason |

### Impact

**Good**:
- Each repository maps 1:1 to a database table — no branching logic in adapters
- Validation pipeline receives two clearly separate inputs
- Test doubles mirror the production boundary exactly
- Adding a new table (e.g. `pending_operation`) means adding a new trait and adapter without touching existing ones

**Bad / Trade-offs**:
- Two repository traits with identical method signatures — some code duplication
- Callers that need both long and short operations must call two repositories separately (though this is always the case in practice)
- Adding a method requires updating both traits and both adapters

### Related

- `src/domain/operation_point.rs` — trait definitions
- `src/infrastructure/pg_adapter.rs` — Postgres implementations
- `src/infrastructure/in_memory_repos.rs` — test doubles
- `src/domain/validation/combined_validation.rs` — validation pipeline
```
