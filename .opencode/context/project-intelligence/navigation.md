<!-- Context: project-intelligence/nav | Priority: high | Version: 1.3 | Updated: 2026-06-28 -->

# Project Intelligence

> Start here for quick project understanding. These files bridge business and technical domains.

## Structure

```
.opencode/context/project-intelligence/
├── navigation.md                  # This file - quick overview
├── domain-model.md                # Ubiquitous language, entities, validations, architecture
├── business-domain.md             # Business context and problem statement
├── business-tech-bridge.md        # How business concepts map to code files
├── decisions-log.md               # Major decisions with rationale
├── companion-backtesting-4.0.md   # Companion repo reference
└── living-notes.md                # Active issues, debt, open questions
```

## Quick Routes

| What You Need | File | Description |
|---------------|------|-------------|
| **Domain glossary & entities** | `domain-model.md` | Ubiquitous language, structs, validation invariants |
| Understand the "why" | `business-domain.md` | Problem, users, value proposition, current state |
| Find the right file | `business-tech-bridge.md` | Business concept → Rust file mapping |
| Know the context | `decisions-log.md` | Why decisions were made |
| Current state | `living-notes.md` | Active issues and open questions |
| **Companion repo** | `companion-backtesting-4.0.md` | Backtesting-4.0 Django backend reference |

## For Agents

**Before writing domain code**: Load `domain-model.md` — it defines the ubiquitous language and validation invariants.

**Before navigating the codebase**: Load `business-tech-bridge.md` — it maps every business concept to a specific file.

## Maintenance

- Update `domain-model.md` when entities, validation rules, or glossary terms change
- Update `business-tech-bridge.md` when new files or features are added
- Update `business-domain.md` when project scope or constraints change
