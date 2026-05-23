<!-- Context: development/backend-navigation | Priority: high | Version: 2.1 | Updated: 2026-05-23 -->

# Backend Development Navigation

**Scope**: Rust server-side, APIs, data layer

---

## Structure

```
development/backend/
├── navigation.md
└── rust/
    ├── actix-patterns.md   # Routes, handlers, app data, error handling
    ├── testing.md          # cargo test, unit/integration tests, fakes
    └── cargo.md            # Cargo commands, Cargo.toml structure
```

---

## Quick Routes

| Task | Path |
|------|------|
| **Actix-web patterns** | `backend/rust/actix-patterns.md` |
| **Testing (cargo test)** | `backend/rust/testing.md` |
| **Package management (cargo)** | `backend/rust/cargo.md` |
| **API design principles** | `principles/api-design.md` |

---

## By Concern

**API / Routes** → `backend/rust/actix-patterns.md`
**Testing** → `backend/rust/testing.md`
**Dependencies** → `backend/rust/cargo.md`

---

## Related Context

- **API Design Principles** → `principles/api-design.md`
- **Core Standards** → `../core/standards/code-quality.md`
