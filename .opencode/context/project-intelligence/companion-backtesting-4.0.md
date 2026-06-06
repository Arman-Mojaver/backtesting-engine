<!-- Context: project-intelligence/companion | Priority: medium | Version: 1.0 | Updated: 2026-06-06 -->

# Companion Repo: Backtesting-4.0

**Location:** `../Backtesting-4.0/` (sibling directory, same parent as backtesting-engine)
**Language:** Python (Django)
**CLI:** `cli/main.py` entry point — commands under `cli/` (strategies, indicators, operation points, raw/resampled points, money management)

## Relationship

backtesting-engine's Rust codebase has a Python/Django counterpart in Backtesting-4.0.
Both repos model the same domain: indicators, strategies, operation points, raw/resampled
points, and money management. Backtesting-4.0 handles persistence, user-facing APIs,
and strategy CRUD; the Rust engine handles data validation and backtest computation.

## Finding Its Context

The companion has its own OpenCode context at:
```
Backtesting-4.0/.opencode/context/
```

Entry point → `Backtesting-4.0/.opencode/context/navigation.md`

When working across both repos, load context from:
1. This project's `.opencode/context/` (backtesting-engine standards)
2. The companion's `.opencode/context/` (Backtesting-4.0 specifics)

## Access

Permission rules in `opencode.json` already allow `**/Backtesting-4.0/**` for read/edit.
