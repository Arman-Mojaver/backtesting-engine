<!-- Context: core/navigation | Priority: critical | Version: 1.1 | Updated: 2026-05-24 -->

# Core Workflows Navigation

**Purpose**: Process workflows for common development tasks

---

## Files

| File | Topic | Priority | Load When |
|------|-------|----------|-----------|
| `code-review.md` | Code review process | ⭐⭐⭐⭐ | Reviewing code |
| `task-delegation-basics.md` | Core delegation workflow | ⭐⭐⭐⭐ | Using task tool |
| `task-delegation-specialists.md` | When to delegate to whom | ⭐⭐⭐⭐ | Choosing specialist |
| `task-delegation-caching.md` | Context caching | ⭐⭐⭐ | Repeated tasks |
| `external-libraries-scenarios.md` | Common scenarios | ⭐⭐⭐ | Examples needed |
| `external-libraries-faq.md` | Troubleshooting | ⭐⭐⭐ | Errors/questions |
| `feature-breakdown.md` | Breaking down features | ⭐⭐⭐⭐ | 4+ files, complex tasks |
| `session-management.md` | Managing sessions | ⭐⭐⭐ | Session cleanup |
| `delegation.md` | Delegation context template | ⭐⭐⭐ | Delegation reference |
| `review.md` | Code review reference | ⭐⭐⭐ | Review reference |
| `external-context-management.md` | Managing external context | ⭐⭐⭐ | External context ops |
| `external-context-integration.md` | Integrating external context | ⭐⭐⭐ | External context setup |

---

## Loading Strategy

**For code review**:
1. Load `code-review.md` (high)
2. Depends on: `../standards/code-quality.md`, `../standards/security-patterns.md`

**For task delegation**:
1. Load `task-delegation-basics.md` (high)
2. Load `task-delegation-specialists.md` (when choosing agent)

**For external libraries**:
1. Reference `external-libraries-scenarios.md` for examples
2. Reference `external-libraries-faq.md` for troubleshooting

**For complex features**:
1. Load `feature-breakdown.md` (high)
2. Depends on: `task-delegation-basics.md`

**For session management**:
1. Load `session-management.md` (medium)

---

## Related

- **Standards** → `../standards/navigation.md`
- **OpenAgents Control Delegation** → `../../openagents-repo/guides/subagent-invocation.md`
