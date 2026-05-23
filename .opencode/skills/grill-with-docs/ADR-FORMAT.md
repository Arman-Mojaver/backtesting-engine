# ADR Format

An ADR (Architecture Decision Record) captures a hard-to-reverse architectural decision with its context and trade-offs. Only write one when all three hold: the decision is hard to reverse, surprising without context, and the result of a real trade-off.

---

## Format

```markdown
# ADR-NNNN: [Short title of the decision]

**Date**: YYYY-MM-DD
**Status**: Accepted | Superseded by ADR-XXXX

## Context

[What situation or problem forced this decision?
What constraints existed? What were we trying to achieve?]

## Decision

[What did we decide? State it clearly and directly.]

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| [Option A] | [Reason] |
| [Option B] | [Reason] |

## Consequences

**Good**: [What this enables or improves]
**Bad / Trade-offs**: [What this costs or constrains]
```

---

## Worked Example

The following is a real ADR from this project:

```markdown
# ADR-0001: Repository Pattern with ABC for Domain/Adapter Separation

**Date**: 2025-01-12
**Status**: Accepted

## Context

The SF1000 KPI calculation pipeline involves several steps (filtering,
delay calculation, censoring dates, modified MIS, final score). All of
these steps operate on `WarrantyPart` and `Vehicle` domain objects.

We needed to decide how the domain logic (calculators) would obtain
warranty part data. The options were to let the calculators query the
database directly, or to introduce an abstraction layer.

The domain logic is the most complex and highest-value part of the
codebase. It must be testable in isolation, without a running Oracle
database.

## Decision

Define `WarrantyPartRepository` as an abstract base class (ABC) in the
domain layer. The domain and use case layer depend only on this
abstraction. The `DBWarrantyPartRepository` in the adapters layer
provides the Oracle/SQLAlchemy implementation.

Tests use `FakeWarrantyPartRepository` (in `src/testing/doubles.rs`),
which holds an in-memory list and never touches the database.

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Calculators query DB directly | Impossible to unit-test without Oracle; couples domain logic to infrastructure |
| Pass raw dicts / DataFrames | Loses type safety; domain logic becomes brittle; no clear contract |
| Use `unittest.mock.patch` on the DB layer | Tests become coupled to internal import paths; break on refactor |

## Consequences

**Good**:
- All domain logic (`SF1000Calculator`, `AverageDelayCalculator`, etc.)
  is fully testable with `cargo test` and no database
- The adapter can be swapped (e.g. in-memory SQLite for integration tests)
  without changing any domain code
- `FakeWarrantyPartRepository` serves as living documentation of the
  contract

**Bad / Trade-offs**:
- Adding a new data access pattern requires updating both the ABC and the
  DB implementation
- The `FakeWarrantyPartRepository` does not filter by `WarrantyPartQuery`
  (it ignores the query), so endpoint tests that rely on filtering must
  pre-seed the exact data they want
```
