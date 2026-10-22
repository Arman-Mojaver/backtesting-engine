---
description: "criterion.rs benchmark session — /bench <function> compares several implementations of one abstraction; /bench <shaA> <shaB> compares two commits"
---

# /bench — criterion.rs performance measurement session

You are executing this repo's benchmark-generation command. Your job: turn the
arguments below into a **verified, runnable criterion.rs session** — code
generated, equivalence and determinism proven, lint-clean, smoke-tested — then
hand the user an exact runbook. You do NOT run the timed benchmark yourself
unless the user accepts your offer to start it in the background.


## 1. Invocation context (captured when this prompt was expanded)

- now = !`date +%Y%m%d_%H%M%S`
- date = !`date +%Y-%m-%d`
- head_full = !`git rev-parse HEAD`
- head_short = !`git rev-parse HEAD | cut -c1-8`
- repo_root = !`git rev-parse --show-toplevel`
- dirty = !`git status --porcelain | head -20`

If any line above is an error or empty, you are not in the git repo root —
stop and tell the user to invoke /bench from the project root.

**Session stamp** = the `now` value above (e.g. `20261022_164512`). It is
embedded in every id and filename this session, so no criterion result can
ever be silently compared against an unrelated one.

## 2. Parsed arguments (substituted by OpenCode — ground truth, never reinterpret)

- FIRST = "$1"
- SECOND = "$2"

Dispatch deterministically on the emptiness of SECOND alone — no name
sniffing, no guessing:

- SECOND empty → **MODE A**: benchmark several implementations of one
  abstraction named by FIRST, all present on the current commit.
- SECOND non-empty → **MODE B**: compare exactly two commits, before = FIRST,
  after = SECOND.

Pre-checks:

- MODE B: both arguments must resolve as commits — run
  `git rev-parse --verify <arg>^{commit}` for each. If either fails, STOP and
  show the two valid invocation forms. Determine order bidirectionally:
  `git merge-base --is-ancestor FIRST SECOND` → FIRST is the before side;
  else `git merge-base --is-ancestor SECOND FIRST` → swap;
  else neither is an ancestor — ask the user which side is the baseline.
  If FIRST or SECOND is HEAD and `dirty` above is
  non-empty, warn that worktrees benchmark the **committed** state only.
- MODE A: if FIRST resolves as a commit hash AND no function/method of that
  name exists, the user probably meant MODE B — say so, ask for the second
  hash, and stop.

## 3. Session identifiers — the no-accidental-comparison rule

Criterion stores every run under `target/criterion/<group>/<function>/<value>/`
and silently compares the next run of the same id against that stored
history. Therefore every id this session encodes **when** it ran and **on
what**:

- MODE A group id: `<subject>_<stamp>_<head_short>`
  (e.g. `sma_20261022_164512_ae440a44`).
- MODE B group id: `<subject>_<stamp>_<before_short>-<after_short>` where
  each short hash is the first 8 chars of the resolved commit id.
- `<subject>`: lowercase `[a-z0-9_]` only — MODE A: the abstraction's short
  name; MODE B: a short name covering the chosen abstraction level (e.g.
  `signal_gen`, `validation`).
- Files: `benches/<subject>_<stamp>.rs`, `tests/equiv_<subject>_<stamp>.rs`
  (+ MODE B per-side snapshots, see §6). Cargo.toml gets
  `[[bench]] name = "<subject>_<stamp>", harness = false` appended.
- MODE B uses the **same** group id and the **same** sub-benchmark ids on
  both sides; which side is which is encoded by the baseline flags, never by
  renaming ids.
- Ids become directory names: lowercase ascii, digits, `_`, `-` only. No
  `/`, spaces, or colons.

## 4. Generated-code conventions (mandatory)

This crate denies `unwrap`, `expect`, `panic`, `indexing_slicing`,
`string_slice`, `as_conversions`, `arithmetic_side_effects` on integers, and
all `pedantic`/`nursery` lints not explicitly allowed in `Cargo.toml`.
Generated files are full crate targets and must pass them. Known pedantic
traps on the pinned toolchain (all hit during framework validation — avoid
them from the first draft):

- `suboptimal_flops` (denied): never `a + b * c` on floats — write
  `b.mul_add(c, a)` (mirror `benches/indicators.rs`); plain `a + b` is fine.
- `format_push_string` (pedantic): never `s.push_str(&format!())` — use
  `use std::fmt::Write;` and `let _ = writeln!(s, ...)`.
- `semicolon_if_nothing_returned` (pedantic): terminate every statement.
- Integer arithmetic is denied (`arithmetic_side_effects`) — use
  `saturating_add`/`saturating_sub`, e.g.
  `values.len().saturating_sub(period.saturating_sub(1))`.
- For MODE B files that must compile identically on BOTH commits: prefer
  universal `From`/`TryFrom` conversions over repo-local helpers (their API
  can differ between eras), and never import
  a crate the diff itself introduced (e.g. itertools on the after side).

Hard rules:

- No `as` casts: use `From`/`TryFrom` or `unwrap_or`/saturating forms. No
  indexing/slicing: `.get()`, iterators.
- No `unwrap()`/`expect()`/`panic!()`; `assert!()`/`assert_eq!()` in tests are fine.
- Prefer `const` items over functions for constants (`missing_const_for_fn`
  is denied). Underscore-separated literals.
- `Result`-returning functions are benchmarked whole: pass the `Result`
  through `black_box` — never unwrap it, never map it to a default.
- Use `std::hint::black_box` (like `benches/indicators.rs` does), wrap both
  inputs and outputs.
- Input generators are **self-contained and deterministic** in every generated
  file (layered-sines style like `synthetic_points`; fixed seeds only; no
  clock, RNG, env, DB, or network). `#[cfg(test)]` fixtures are invisible to
  `benches/` and `tests/` — never import them.
- Inputs are built once, outside the timed loop. Read-only inputs (`&[..]`):
  `b.iter(|| black_box(f(input)))`. Consuming/mutating inputs:
  `b.iter_batched(|| input.clone(), |fresh| black_box(f(&fresh)), BatchSize::SmallInput)`.
  Set `group.throughput(Throughput::Elements(n))` when per-iteration work
  scales with input length.
- Async functions only: `b.to_async(TokioExecutor)` and extend the criterion
  dev-dependency features with `async_tokio`.
- Never modify `src/`, existing tests, or `Cargo.lock` by hand. New
  code goes only into the files named in §3. Cargo.toml edits: append the
  per-session `[[bench]]` block; this checkout sets `autobenches = false`,
  so only explicitly registered bench targets build there. In a worktree
  whose manifest lacks that key, check for other unregistered `.rs` files in
  `benches/` and report them instead of silently breaking them. Add the
  criterion dev-dependency
  `criterion = { version = "0.8", features = ["html_reports"] }` only if
  that side of the comparison lacks it.
- Always invoke cargo bench as `cargo bench --bench <name> -- <flags>` —
  never bare `cargo bench -- <flags>` (libtest's harness intercepts criterion
  flags like `--save-baseline` and aborts the run).

## 5. MODE A — one abstraction, several implementations

FIRST names one code abstraction: a free function (`sma`), a path
(`moving_averages::sma`), or a method (`Trait::method`). The repo already
holds 2..N implementations of it on the current commit — usually in
different files. Prove they are business-logic equivalent, then race them.

1. **Discover** every implementation: grep for `fn <name>` (or, for methods,
   every `impl Trait for ..` block defining it). Report each with
   file:line and signature. Fewer than two implementations → stop (nothing
   to compare; hint at MODE B). More than four, or matches that are clearly
   different abstractions sharing a name → list them and ask which count.
2. **Input model**: read each implementation's docs and tests; choose a
   shared input domain. Pick 2–4 realistic scales for the benchmark, plus
   edge cases (empty, minimal, boundary) for the equivalence test only.
   If signatures differ slightly (e.g. one takes `&[f64]`, another takes
   `&[ResampledPointD1]`), adapt the same underlying data to each and
   document the mapping.
3. **Equivalence test** `tests/equiv_<subject>_<stamp>.rs` per §7:
   determinism (each implementation run twice per input) + pairwise
   cross-implementation comparison. Run it
   (`cargo test --test equiv_<subject>_<stamp>.rs -- --nocapture`).
   On failure: report the exact per-input deltas and STOP — never loosen
   tolerance silently. On pass: keep the observed max-delta table for the
   final report.
4. **Bench file** `benches/<subject>_<stamp>.rs` per §8: one
   `BenchmarkGroup` with the §3 group id; one sub-benchmark per
   (implementation, scale) — `BenchmarkId::new("<impl_short>", <scale>)`.
   `<impl_short>` distinguishes implementations (function name, or type name
   for trait methods).
5. **Register** the `[[bench]]` block in Cargo.toml (dev-dep only if
   missing).
6. **Verify pipeline** §9, then **final report + runbook** §10.

## 6. MODE B — two commits

Changes between the two commits may be spread across many functions and data
structures — there is no single "the function". You must **choose the right
abstraction level**: the narrowest public entry point(s) that (a) exercise
every changed code path and (b) exclude everything unchanged or unrelated.
Exactly two sides: before and after.

1. **Validate hashes and order** per §2.
2. **Read the diff**: `git diff --stat <before> <after>` then
   `git diff <before> <after> -- src/ Cargo.toml`. Classify every changed
   item: pure computation / IO+DB / config+build / docs+tests. IO, DB,
   config, and docs changes are excluded from benchmarking — state that
   explicitly in the report. If, after exclusion, **no pure-computation
   change remains** (e.g. the pair diff is a config value or a dependency
   bump), STOP: there is nothing meaningful to benchmark — tell the user
   what the diff actually contains and suggest what to compare instead.
3. **Choose the abstraction level** (the core reasoning step — show your
   work):
   - Map each changed pure-computation item to its nearest **public** entry
     point by following callers upward within `src/`.
   - All changes funnel into one pub fn → that fn is the level.
   - Changes form K independent clusters with no narrow common caller (or the
     only common caller drags in unrelated IO, like the Postgres-backed
     strategy pipeline) → the level is the **set** of those K pub entry
     points; benchmark each as its own sub-benchmark family in one group.
   - Prefer the lowest pub entry that covers a cluster; go higher only when
     lower entries do not exercise the changed paths.
   - Produce a **coverage matrix**: changed item → which benchmark exercises
     it → rationale. Anything uncovered must be justified in writing. If
     material pure-computation changes are uncoverable, say so and ask.
   - Ambiguous (two plausible levels): present both with trade-offs and ask.
     Otherwise state the choice and rationale, then continue.
4. **Inputs**: for each entry point, deterministic self-contained inputs at
   2–3 scales. Include inputs that exercise changed branches (if a change
   touches an empty-input path, the equivalence test must contain that case).
5. **Worktrees** (the main checkout is never touched):
   `git worktree add <repo_root>/.tmp/bench-sessions/<stamp>/before <sha>`
   and `.../after <sha>`. `.tmp` is gitignored.
6. **Generate in both worktrees** the **same** bench file and equivalence
   test (§3, §7, §8), plus the Cargo.toml registration (older commits may
   lack the criterion dev-dependency — add the exact §4 line). Sub-benchmark
   ids are `(entry_point_short, scale)` — identical on both sides.
   Exception: if the abstraction's public signature itself changed between
   the commits, identical files cannot compile on both sides — generate
   per-side adapted files with the same ids, and document the signature
   change prominently (it is equivalence-relevant too).
7. **Equivalence, three layers**:
   - per-side determinism (the test runs each entry point twice per input);
   - snapshots: in each worktree run
     `EQUIV_SNAPSHOT=<repo_root>/.tmp/bench-sessions/<stamp>/equiv-<side>.txt cargo test --test equiv_<subject>_<stamp> -- --nocapture`
     — the test appends one canonical line per input
     (`<label>\t<output>`, `serde_json::to_string` when the output type
     derives Serialize, else `{:?}` — never panic on serialization failure;
     fail the test instead);
   - cross-commit: `diff` the two snapshot files. Identical → pass. If lines
     differ, read both files and compute the deltas yourself: float-only
     differences within §7 tolerance → pass, reported as a note with the
     numbers; any discrete difference (lengths, timestamps, variants) or
     out-of-tolerance floats → report exactly and STOP — a real behaviour
     change makes the performance comparison apples-to-oranges, and the user
     decides whether to proceed.
8. **Verify pipeline** §9 in each worktree with
   `CARGO_TARGET_DIR=<repo_root>/target` (shared: dependency artifacts
   compiled once, criterion data store shared between the two sides).
9. **Final report + runbook** §10, including the coverage matrix and the
   worktree cleanup commands.

## 7. Equivalence specification (shared by both modes)

- Integer, bool, string, enum, and composite outputs of those: exact
  `assert_eq!()`.
- `f64`: equivalent iff `|a - b| <= ATOL + RTOL * max(|a|, |b|)` with
  `ATOL = 1e-12`, `RTOL = 1e-9`; both-NaN counts as equal; `-0.0`/`0.0`
  equal. Env `EQUIV_EXACT=1` sets both tolerances to zero (IEEE equality).
  Never change the defaults silently.
- `Result<T, E>`: compare payloads when both are `Ok`; same error variant
  when both are `Err`. One side `Ok` and the other `Err` on the same input is
  a failure, reported loudly.
- Sequences/structs: element/field-wise with the above semantics, and
  lengths/field counts must match exactly.
- Ordering-unspecified outputs (e.g. HashMap iteration order): canonicalise
  with a stable sort before comparing and say so.
- Clock, RNG, env, or DB dependence: inject determinism (fixed seed, frozen
  time, in-memory data) if the pub API allows; otherwise STOP and explain.
- The harness must print a per-input table — input label × pair (or run) ×
  max relative delta — and collect mismatches into one report string, then
  `assert!(mismatches.is_empty(), "{report}")`. It never panics outside that
  assert.

## 8. Bench-file shape

One file, `benches/<subject>_<stamp>.rs`, header doc comment recording the
session facts (date, subject, commit hashes, group id), then:

```rust
use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};

// --- TUNING (raise for publication-grade numbers) ---
const WARM_UP: Duration = Duration::from_secs(1);
const MEASURE: Duration = Duration::from_secs(3);

// deterministic, self-contained input generators here

fn bench_<subject>(c: &mut Criterion) {
    let mut group = c.benchmark_group("<group id from §3>");
    group.warm_up_time(WARM_UP);
    group.measurement_time(MEASURE);
    for scale in [/* 2-4 scales */] {
        let input = /* generated once, per scale */;
        group.throughput(Throughput::Elements(/* elements in input */));
        // MODE A: one bench_with_input per (impl, scale).
        // MODE B: one bench_with_input per (entry point, scale); the SAME
        // ids run on both sides; `iter` vs `iter_batched` per §4.
        group.bench_with_input(BenchmarkId::new("<impl or fn short>", scale), &input, |b, i| {
            b.iter(|| black_box(/* call */));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_<subject>);
criterion_main!(benches);
```

## 9. Verification pipeline (you run these; fix the generated code until all four pass)

MODE A: in the main checkout. MODE B: in each worktree, with
`CARGO_TARGET_DIR=<repo_root>/target` set.

1. `cargo fmt`
2. `cargo clippy --bench <subject>_<stamp> --test equiv_<subject>_<stamp> -- -D warnings`
3. `cargo test --test equiv_<subject>_<stamp> -- --nocapture`
   (MODE B: with the `EQUIV_SNAPSHOT` env of §6)
4. `cargo test --bench <subject>_<stamp>` — criterion test mode: compiles
   and runs each routine once without measuring (the compile-and-run-once
   gate from the criterion guide).

## 10. Runbook — always the final section of your report

Substitute real values; never placeholders.

**MODE A**

```text
cargo bench --bench <subject>_<stamp> -- --verbose
# repeat the same command to compare against this session's previous run —
# the session-stamped id guarantees it can only ever be compared with itself
```

- Report: `target/criterion/report/index.html` → group
  `<subject>_<stamp>` — violin plots and line charts compare the
  implementations directly across scales.
- Expected wall time ≈ build + (implementations × scales) × (~WARM_UP + MEASURE + analysis).
- Environment discipline: quiet machine, charger plugged in, no parallel
  builds or containers, run 2–3 times, treat differences inside criterion's
  noise threshold as ties.
- Cleanup: delete `benches/<subject>_<stamp>.rs` and
  `tests/equiv_<subject>_<stamp>.rs`, remove the `[[bench]]` block, and
  optionally `rm -rf target/criterion/<subject>_<stamp>_*`.

**MODE B** (order matters; before side first, after side last)

```text
cd <repo_root>/.tmp/bench-sessions/<stamp>/before
CARGO_TARGET_DIR=<repo_root>/target cargo bench --bench <subject>_<stamp> -- --verbose --save-baseline before

cd <repo_root>/.tmp/bench-sessions/<stamp>/after
CARGO_TARGET_DIR=<repo_root>/target cargo bench --bench <subject>_<stamp> -- --verbose --baseline before
```

- The after-side output prints `change: [...]` per benchmark with
  "Performance has improved / regressed / no change"; the shared
  `CARGO_TARGET_DIR` makes the before side's baseline visible to the after
  side, and the session-stamped group id keeps everything isolated from
  unrelated runs.
- Report: `<repo_root>/target/criterion/report/index.html`.
- Cleanup: `git worktree remove --force` both worktree paths, then
  `rm -rf <repo_root>/.tmp/bench-sessions/<stamp>` (keep the two snapshot
  files if the user wants the equivalence evidence).

**Both modes** — end with: "I can start the benchmark now in the background
and report the numbers when it finishes — want me to?"

## 11. Guardrails

- Never modify `src/`, existing tests, `docs/`, or hand-edit `Cargo.lock`.
- Never silently benchmark through IO/DB/network; if the only covering entry
  point is IO-bound, say so and ask.
- Never invent implementations — MODE A benchmarks what already exists.
- Never loosen equivalence semantics or hide a mismatch; a mismatch is a
  finding, reported first, and the session stops there.
- Never drop the stamp or commit-hash parts of an id, and never reuse a
  session id.
- Keep the final report lean: discovery summary, equivalence table, coverage
  matrix (MODE B), chosen level + rationale (MODE B), runbook, background
  offer.
