<!-- Context: development/navigation | Priority: critical | Version: 2.3 | Updated: 2026-05-24 -->

# Development Navigation

**Purpose**: Software development across the Rust backend stack.

---

## Structure

```
development/
├── navigation.md
├── backend-navigation.md      # Shortcut to backend patterns
│
├── principles/                # Universal (language-agnostic)
│   ├── navigation.md
│   ├── clean-code.md
│   └── api-design.md
│
├── backend/                   # Rust server-side
│   ├── navigation.md
│   └── rust/
│       ├── testing.md
│       ├── cargo.md
│       └── errors/            # Rust compiler error reference
│           ├── index.md       ← START HERE for any compiler error
│           ├── ownership-borrow.md
│           ├── lifetimes.md
│           ├── traits.md
│           ├── types.md
│           ├── patterns.md
│           ├── modules.md
│           ├── generics.md
│           └── async.md
│
├── data/                      # Database / ORM
│   └── sqlalchemy-oracle.md
│
├── integration/               # Connecting systems
│   └── navigation.md
│
└── infrastructure/            # DevOps
    └── navigation.md
```

---

## Quick Routes

| Task | Path |
|------|------|
| **Clean code** | `principles/clean-code.md` |
| **API design** | `principles/api-design.md` |
| **SQLAlchemy / Oracle DB** | `data/sqlalchemy-oracle.md` |
| **Rust compiler error** | `backend/rust/errors/index.md` |
| **Ownership/borrow/move errors** | `backend/rust/errors/ownership-borrow.md` |
| **Lifetime errors** | `backend/rust/errors/lifetimes.md` |
| **Trait bound errors** | `backend/rust/errors/traits.md` |
| **Type mismatch errors** | `backend/rust/errors/types.md` |
| **Pattern match errors** | `backend/rust/errors/patterns.md` |
| **Import/visibility errors** | `backend/rust/errors/modules.md` |
| **Generic/assoc type errors** | `backend/rust/errors/generics.md` |
| **Async/await errors** | `backend/rust/errors/async.md` |

---

## By Concern

**Principles** → Universal development practices (language-agnostic)
**Data** → SQLAlchemy, Oracle DB patterns
**Rust Errors** → Compiler error lookup by code (E0XXX → category file → exact fix)
**Integration** → APIs, third-party services
**Infrastructure** → Docker, CI/CD

---

## Related Context

- **Core Standards** → `../core/standards/navigation.md`
- **RustErrorScout subagent** → `.opencode/agent/subagents/development/rust-error-scout.md`
