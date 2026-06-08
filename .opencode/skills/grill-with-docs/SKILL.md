---
name: grill-with-docs
description: Grilling session that challenges your plan against the existing domain model, sharpens terminology, and updates documentation (domain-model.md, decisions-log.md) inline as decisions crystallise. Use when user wants to stress-test a plan against their project's language and documented decisions.
disable-model-invocation: true
---

Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.

Ask the questions one at a time, waiting for feedback on each question before continuing.

If a question can be answered by exploring the codebase, explore the codebase instead.

## Domain awareness

During codebase exploration, also look for existing documentation:

### File structure

This project stores its domain documentation in `.opencode/context/project-intelligence/`. The key files are:

```
/
├── .opencode/
│   └── context/
│       └── project-intelligence/
│           ├── domain-model.md           ← Glossary & ubiquitous language
│           ├── business-domain.md         ← Business context
│           ├── decisions-log.md           ← Architecture decisions (like ADRs)
│           └── business-tech-bridge.md    ← Business concept → Rust file mapping
└── src/
```

The glossary in `domain-model.md` is the single source of truth for domain language. The `decisions-log.md` captures hard-to-reverse architectural decisions with their context and trade-offs.

Update these files lazily — only when you have something to write. If `domain-model.md` exists, add new terms to its glossary section. If `decisions-log.md` exists, append new decision entries.

## During the session

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in `domain-model.md`, call it out immediately. "The glossary in domain-model.md defines 'instrument' as the trading pair, but you seem to mean the underlying asset — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'strategy' — do you mean the Money Management Strategy or the Trading Strategy? Those are different things."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it: "Your code filters OperationPoints by mms_id, but you just said all operations for an instrument are included — which is right?"

### Update domain-model.md inline

When a term is resolved, update the glossary in `domain-model.md` right there. Don't batch these up — capture them as they happen. Use the format in [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md).

Don't couple the glossary to implementation details. Only include terms that are meaningful to domain experts.

### Log decisions in decisions-log.md

When a hard-to-reverse decision is made, append a new entry to `decisions-log.md`. Only offer to log a decision when all three are true:

1. **Hard to reverse** — the cost of changing your mind later is meaningful
2. **Surprising without context** — a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off** — there were genuine alternatives and you picked one for specific reasons

If any of the three is missing, skip the entry. Use the format in [ADR-FORMAT.md](./ADR-FORMAT.md).
