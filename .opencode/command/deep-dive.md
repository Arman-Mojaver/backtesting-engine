---
description: Generate an educational walkthrough script for code — from a commit OR arbitrary input
tags: [deep-dive, walkthrough, learning, code-analysis, rust]
dependencies:
  - skill:deep-dive
---

# Deep Dive Command

Generate a runnable, interactive walkthrough script that helps developers understand implementation
details beyond the surface description. Works in two modes.

<critical_rules priority="absolute" enforcement="strict">
  <rule id="mode_detection">
    MUST detect mode before anything else: commit-based vs code-input. Never guess — if ambiguous, ask.
  </rule>
  <rule id="load_skill">
    MUST load the deep-dive skill (.opencode/skills/deep-dive/SKILL.md) and follow its rules for
    script generation (interactive sections, searchable titles, output limits, CLI/API safety).
  </rule>
  <rule id="side_effect_free">
    Walkthrough scripts MUST be side-effect-free: never execute CLI commands or make real HTTP calls.
    Print instructions instead.
  </rule>
  <rule id="runnable">
    Generated scripts MUST compile and run immediately via `cargo run --example <name>`.
  </rule>
</critical_rules>

**Arguments**: `$ARGUMENTS`

---

## Usage

```bash
/deep-dive                          # Commit mode — analyze the last commit
/deep-dive --code <code>            # Code input mode — analyze pasted code
/deep-dive --code <file>:<symbol>   # Code input mode — analyze a struct/fn in a file
/deep-dive <struct|function name>   # Code input mode — find & analyze a symbol
```

---

## Mode Detection

Determine the mode from `$ARGUMENTS`:

| Arguments | Mode |
|-----------|------|
| empty, or "commit", or "last" | **Commit Mode** |
| `--code` flag present | **Code Input Mode** |
| pasted code block follows | **Code Input Mode** |
| a struct/function/file reference (e.g. `SignalGenerator`, `src/foo.rs:Bar`) | **Code Input Mode** |
| ambiguous → ask: "Commit mode or code input mode?" | Wait for user |

---

## Mode A: Commit Analysis

<workflow id="commit_mode">
  <stage id="1" name="CheckUncommitted">
    Run `git status --porcelain`. If uncommitted changes exist, warn the user that temporary
    debug print statements (if added) will mix with their changes. Offer: commit / stash / proceed.
  </stage>

  <stage id="2" name="AnalyzeCommit">
    Run `git show HEAD --stat` and `git show HEAD`. Identify added/modified structs, enums,
    functions, their methods, code paths, and dependencies.
  </stage>

  <stage id="3" name="GenerateScript">
    Follow the deep-dive skill's script-generation rules: seed data, instantiate at multiple
    levels, traverse code paths, interactive numbered sections with searchable titles, 20-25 line
    output limit, CLI/API safety. Save to `examples/deep-dive-{timestamp}-{slug}.rs`.
  </stage>

  <stage id="4" name="PresentResults">
    Show the Output Format (commit variant) from the skill: script path, commit info, sections,
    run instructions, cleanup note.
  </stage>
</workflow>

---

## Mode B: Code Input Analysis

<workflow id="code_input_mode">
  <stage id="1" name="ReceiveCode">
    Accept code in any form:
    - Pasted code block (any language, any size)
    - File path + symbol: `src/application/signal_generator.rs:SignalGenerator`
    - Bare symbol name: `SignalGenerator` (search the codebase)
  </stage>

  <stage id="2" name="ResolveCode">
    If a symbol + file is given, read the file and locate the struct/fn. If a bare symbol is given,
    search the codebase (`rg`, grep, glob) to find its definition. If ambiguous (multiple matches),
    list candidates and ask the user to pick one.
  </stage>

  <stage id="3" name="AnalyzeCode">
    Parse the provided/resolved code. Identify structures, methods, visibility, code paths
    (happy/edge/error/branching), and CLI/API patterns. Read surrounding context for imports and
    module structure so the generated script's `use` statements are correct.
  </stage>

  <stage id="4" name="GenerateScript">
    Same script-generation rules as commit mode. Save to `examples/deep-dive-{timestamp}-{slug}.rs`.
    The script imports the target symbols from the project (or defines them locally if pasted code
    is self-contained).
  </stage>

  <stage id="5" name="PresentResults">
    Show the Output Format (code-input variant) from the skill.
  </stage>
</workflow>

---

## Script Generation Rules (both modes)

Defer to `.opencode/skills/deep-dive/SKILL.md`. Summary of the key constraints:

1. **Runnable** — compiles with `cargo run --example <name>` without extra setup.
2. **Interactive sections** — `main()` dispatches numbered sections via `std::env::args()`.
3. **Searchable titles** — every section prints its exact title so the user can search the file.
4. **Output limit** — max 20-25 printed lines per section.
5. **Seed data** — realistic hardcoded fixtures.
6. **Code paths** — happy path, edge cases, error conditions, branching.
7. **CLI/API safety** — print commands/curl, never execute.
8. **Split if large** — >200 lines → split into multiple focused scripts.
9. **Language** — Rust (this project); separate scripts for other languages if needed.

---

## Examples

### Commit Mode
```bash
/deep-dive
# → analyzes `git show HEAD`, generates examples/deep-dive-20260812-sma-cross.rs
```

### Code Input Mode — pasted code
```bash
/deep-dive --code
pub struct RiskManager { max_drawdown: f64, ... }
# → generates examples/deep-dive-20260812-risk-manager.rs
```

### Code Input Mode — file + symbol
```bash
/deep-dive src/application/signal_generator.rs:SignalGenerator
# → reads the file, locates SignalGenerator, generates walkthrough
```

### Code Input Mode — bare symbol
```bash
/deep-dive BacktestRunner
# → searches codebase, finds the definition, generates walkthrough
```

---

## Success Criteria

- [ ] Mode correctly detected (never guessed)
- [ ] Script compiles and runs (`cargo run --example <name>`)
- [ ] Sections are interactive and searchable
- [ ] Each section ≤ 25 printed lines
- [ ] CLI/API commands printed, never executed
- [ ] Cleanup instructions provided (`rm examples/deep-dive-*.rs`)
