---
name: improve-code
description: Read-only review of a single Rust target (function, struct, trait, abstraction, module or file). Finds beginner- and mid-level practices, test gaps, crate misuse and readability issues, and ranks them by impact/effort against the bar of a senior production Rust engineer. Informative only, never edits code. Use when the user says "improve code", "improve this", "senior review", "how senior is this", "review this struct/function/file", or names one piece of Rust code to assess.
argument-hint: <path | path::Item | abstraction name>
---

# Improve Code (review only)

Assess ONE target against senior production-Rust quality. Do not review the whole project.

## Ground rules

- **Informative only.**
  - Do not edit, create or delete any file.
  - Do not run formatters, fixers or any command that writes to disk.
  - Read-only commands are allowed, such as `cargo clippy` or `cargo test`, but not `cargo fix` or `cargo fmt`.
- **Do not plan follow-up work for yourself.** That means:
  - no task lists or TODO files
  - no "next I will…" plans
  - no offer to apply the fixes
- The output is a report for a human, who decides what to do with it.
- **The bar:** if every finding were resolved, a senior Rust engineer reading the target would conclude that a senior Rust engineer wrote it.
- Only recommend changes that make the code simpler, clearer, or more correct. Never recommend cleverness or extra layers just to look senior.
- **Every finding is one small commit.**
  - Think of a ~400-line PR split into 5–10 coherent, targeted commits. Each finding must be the size of one of those commits.
  - One concern per finding: one rename, one extraction, one error type, one test group.
  - Small diff: usually under ~60 changed lines, about 5–10 minutes of work.
  - It can be applied on its own and still compile and pass the tests.
  - The whole list should be something a human can mostly implement in about 1 hour.
  - If an improvement is bigger than that, split it into a sequence of commit-sized findings, ordered, each one safe to stop after.
  - If it can't be split, put it under **Larger refactors** in the report (see step 5) instead of the main list.
- **Respect intentional exceptions.** These are deliberate choices. Do not report them as findings:
  - code marked `#[expect(<lint>, reason = "...")]` or `#[allow(<lint>, reason = "...")]`
  - code marked with an `// Intentional: ...` comment
  - lints disabled or relaxed in `[lints]` in `Cargo.toml`, or in `clippy.toml`
  - Exceptions to the exceptions:
    - If an intentional exception causes a concrete bug, report it under **Not recommended** as "challenges intentional exception at `file:line`", with the evidence.
    - A bare `#[allow(...)]` with no `reason` is not an intentional exception. It may be reported (e.g. "missing reason", or "switch to `#[expect]`").

## 1. Resolve the target

- Target = `$ARGUMENTS`.
- If it is empty or ambiguous, ask the user which function, struct, abstraction or file to review. Do not guess.
- Find the target's definition and write down its boundary:
  - the items it owns
  - its public API
  - its callers (grep for usages)
  - its tests
- Code outside the target is context only. Mention it only when it directly affects the target's quality (e.g. callers that work around a bad signature).
- **Size limit.** If the target is bigger than about 500 lines or spans several modules:
  - do not review it
  - suggest 3–6 smaller targets (specific structs, functions or files), each with one line on why it's a good unit
  - stop there
  - A shallow review of a big target is worse than a deep review of a small one.

## 2. Gather context

- Read `.opencode/context/project-intelligence/`:
  - `domain-model.md`: domain language and concepts
  - `decisions-log.md`: past trade-offs. Flag any recommendation that contradicts a recorded decision.
  - `living-notes.md`: current state and known issues
- Skim `.opencode/context/core/standards/` and `.opencode/context/development/principles/` for project conventions.
- Read the project config files that apply to the target:
  - `Cargo.toml`
  - `clippy.toml`
  - `rustfmt.toml`
  - `deny.toml`
  - `rust-toolchain.toml`
  - `Makefile`
  - CI config
- Read the callers and tests from step 1. How the target is used decides what a good abstraction looks like.
- Find the sibling code that already does this well, and use it as the reference for project consistency.
- **Crates used by the target.** For each external crate the target imports or touches:
  - Find its version and its enabled features (default and explicit) in `Cargo.toml` / `Cargo.lock`.
  - Learn its public API for that exact version:
    - main types, traits, builders and error types
    - extension traits
    - anything feature-gated
  - Where to look, in order:
    - the `context7` skill
    - the crate's source in `~/.cargo/registry/src/`
    - `cargo doc --no-deps -p <crate>` (read-only; writes only to `target/`)
  - Note every other place in the project that uses the same crate, so the review sees how much of it the project really uses overall.
  - Do not assess crate usage from memory. Versions change APIs.

- **Run clippy for real.** Do not guess what clippy would say.
  - Run `cargo clippy --all-targets` and keep only the warnings inside the target.
  - Optionally run `cargo clippy --all-targets -- -W clippy::pedantic` and keep the extra warnings for the target separately. These are advisory only.
  - Both commands write only to `target/` and are allowed.

## 3. Evaluate the target on every axis

For each axis, note concrete findings only, each pointing to `file:line`. Skip an axis if nothing applies.

- **Backtesting-engine domain checks** (check these first, because they are the highest-value bugs in this project)
  - **Look-ahead bias and off-by-one in time series.** This is the most important check.
    - Does any computation at bar `i` read data from `i+1` or later?
    - Are `[i]` and `[i-1]` used correctly for crossovers and signals?
    - Are indicator warm-up periods respected? (No output before enough bars exist; no silent zeros or NaNs in that window.)
    - Are signals acted on at the right bar (signal on close of `i`, fill at `i+1`)?
  - **Numeric correctness**
    - `f64` vs. a decimal type for prices, money and PnL
    - NaN and infinity handling and propagation
    - float comparisons with `==` or without a tolerance
    - rounding error that adds up over long series
    - division by zero (e.g. zero volume, a flat price series)
  - **Determinism.** The same input must give the same output. Look for:
    - `HashMap`/`HashSet` iteration order affecting results
    - wall-clock time (`SystemTime::now`, `Utc::now`) inside logic
    - randomness without a seed
    - parallel reductions whose float results depend on order
  - **Parquet / Arrow usage**
    - Is data processed by column, or converted row by row?
    - Are record batches or columns materialized (collected or cloned) more than once?
    - Is only the data that's needed being read? (projection and predicate pushdown)

- **Abstractions**
  - Is this the right abstraction, or is it leaking or wrong-sized?
  - Is a trait used where a plain function or enum would do?
  - Are generics or `dyn` justified?
  - Are newtypes used for domain values (prices, timestamps, ids) where they prevent mix-ups?
- **Single level of abstraction**
  - Each function should read at ONE level of abstraction. Either it describes *what* happens (a sequence of well-named steps), or it implements *how* (the low-level details). It should not mix both.
  - Flag functions that interleave high-level orchestration with low-level details. Examples:
    - parsing bytes, index math or string formatting sitting next to domain steps
    - a `match` on raw values in the middle of business flow
  - Recommend extracting the lower-level parts into well-named functions so the parent reads like a narrative.
  - **This is NOT over-engineering.** The extraction exists for a human reader, even if the extracted function has only one caller. Do not list it under over-engineering or reject it for having one caller.
  - The same applies to modules and structs: a type should not mix domain concepts with plumbing details.
- **Idiomatic Rust**
  - ownership and borrowing: needless `clone()`, `&String`/`&Vec<T>` instead of `&str`/`&[T]`
  - iterators vs. index loops
  - `Option`/`Result` combinators
  - `impl From`/`TryFrom`, `Default`, `Display`
  - derive hygiene
  - pattern matching
  - `#[must_use]`
  - visibility (`pub` vs. `pub(crate)`)
  - **Lifetimes**
    - explicit lifetimes that elision already covers
    - `'static` bounds that are too strict and force needless `clone()`, `String` or `Arc`
    - structs holding references where owning the data would be simpler (or the other way round)
  - **Generics cost**
    - generic code (or `impl Trait` in many places) that gets compiled again for each concrete type, slowing compile times or bloating the binary for no real benefit
    - Consider a non-generic inner function, `&dyn Trait`, or a concrete type.
    - Keep the generic when it is on a hot path where static dispatch measurably matters.
- **API design** (against the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html))
  - **C-CONV:** conversion naming. `as_` is cheap and borrowed, `to_` is expensive, `into_` consumes self.
  - **C-GETTER:** getters are named `field()`, not `get_field()`.
  - **C-COMMON-TRAITS:** public types implement the common traits that make sense for them:
    - `Debug`
    - `Clone`
    - `PartialEq`/`Eq`
    - `Hash`
    - `Default`
    - `Display`
    - `Send`/`Sync` where expected
  - Other guidelines worth checking:
    - **C-CTOR:** constructors are `new` / `with_*` / `from_*`.
    - **C-ITER:** iterator methods are named `iter`, `iter_mut`, `into_iter`.
    - **C-VALIDATE:** arguments are validated, or the type makes invalid values impossible.
  - Cite the guideline ID in the finding (e.g. `[api: C-GETTER]`).
- **Crate usage**
  - **Use what the crate offers.** Flag places where the target:
    - converts the crate's types into hand-rolled ones (or the other way round) for no reason
    - ignores the crate's own data structures, traits or error types
    - fights its intended usage
  - **Hand-rolled reimplementation.** Flag code that does by hand something the crate already provides, such as:
    - parsing
    - date math
    - iterator adapters
    - retries
    - serialization helpers
    - Recommend the crate API by name, for the version in use.
  - **Bloat (the opposite case).** Flag when a crate, or a feature of it, is imported for a single small abstraction that would be simple to write by hand. Example: a whole crate pulled in for one helper, or default features enabled that nothing uses. Recommend one of:
    - narrowing features (`default-features = false` plus only what's needed)
    - replacing it with a small local function
    - removing the dependency
    - State the trade-off: compile time, binary size, supply chain (`deny.toml`) vs. correctness, maintenance and edge cases the crate already handles.
  - **Balance, not dogma.**
    - Keep a crate when reimplementing it would be subtle or error-prone (time zones, float formatting, crypto, parsing, concurrency primitives), or when the project already uses it widely.
    - Recommend adopting more of a crate only when that removes real code or real bugs. Doing it just because the crate can is not a reason.
    - Recommend dropping a crate only when the replacement is clearly small, obvious and safe.
    - When it is a judgment call, say so and give both sides, instead of picking a side.
- **Performance**
  - allocations in hot paths
  - `Vec::with_capacity`
  - copying large data
  - unnecessary `Arc<Mutex<_>>`
  - async blocking calls
  - `collect` followed by re-iterating
  - Only flag what matters on the target's real call path.
- **Error handling**
  - `unwrap`/`expect`/`panic!` in non-test code
  - stringly-typed errors
  - lost error context
  - a library-style typed error (`thiserror`) vs. an application-style one (`anyhow`) used in the wrong place
  - errors swallowed with `let _ =` or `.ok()`
- **Recoverability**
  - What happens on partial failure?
  - Can it retry or resume?
  - Are invariants preserved when it fails halfway?
  - Is cleanup handled (RAII/`Drop`)?
- **Logging / observability**
  - right level
  - structured fields instead of string-formatted messages
  - `tracing` spans where useful
  - no noise in hot loops
  - consistent with the existing log style
- **Testing: quality**
  - tests coupled to internals instead of behavior
  - fakes vs. mocks (`src/application/fake_repositories.rs`)
  - unclear test names
  - duplicated setup that should be a helper
  - assertions too weak to catch regressions
- **Testing: gaps** (always report this section, even when it's empty)
  - List every behavior of the target that has no test. Check each of these:
    - each public function or method
    - each branch or `match` arm with distinct behavior
    - each error path
    - boundary values: empty input, single element, zero, negative, max, NaN for floats
    - invariants the type promises
  - For each gap, give:
    - the behavior to cover
    - why it matters (what bug it would catch)
    - a suggested test name, e.g. `macd_returns_none_when_series_shorter_than_slow_period`
  - Point out where a property test or doctest would cover a class of cases better than examples would.
- **Readability & simplicity**
  - naming in the domain language
  - function length
  - nesting depth (use early returns)
  - magic numbers
  - comments that explain *what* instead of *why*
- **Over-engineering**
  - layers, traits, generics, builders or config with only one user
  - abstractions written for futures that don't exist
  - Extracting functions to keep a single level of abstraction does not count (see above).
- **Dead code**
  - unused functions, fields, variants or params
  - `#[allow(dead_code)]`
  - commented-out code
  - stale feature flags
- **DRY**
  - duplicated logic inside the target or between the target and its siblings
  - Only recommend extracting when the duplicates truly share meaning, not just shape.
- **Refactors that simplify**
  - a change that removes concepts, branches or types
  - "make illegal states unrepresentable" via types or enums
- **Mismatches / misalignment**
  - code vs. domain model
  - name vs. behavior
  - signature vs. what callers actually need
  - inconsistency with sibling modules
- **Documentation**
  - stale docs, comments or README sections about the target
  - missing `///` on the public API
  - missing `# Errors` / `# Panics` sections
- **Trade-offs**
  - Is each choice consistent with the trade-offs in `decisions-log.md`?
  - Any trade-off made silently that should be recorded?
- **Architecture fit**
  - Does the target sit in the right layer (domain / application / infrastructure)?
  - Do dependencies point the right way?
- **Tooling / CI**
  - lints that would have caught these issues
  - Should a clippy lint, `deny.toml` rule, or CI step be added so the issue can't come back?

## 4. Verify before reporting (avoid false positives)

- Re-open the file and confirm every `file:line` reference points at the code described. Line numbers must come from reading the file, never from memory or estimation. Fix or drop any reference that doesn't match.
- Confirm every claim about a crate API against the version in use (step 2).
- Confirm every test gap by searching for tests in all of these places before reporting it:
  - `#[cfg(test)]` modules
  - `tests/`
  - `benches/`
  - doctests
- Drop any finding you can't back with concrete code.

## 5. Report

The report is the only output. Format:

- **Target:** what was reviewed and its boundary (one or two lines)
- **Verdict:** one paragraph. What level it reads at today (beginner / mid / senior) and the biggest thing keeping it from senior.
- **Findings, ranked.** Each one is a single commit-sized change:
  - Use one bullet per finding:
    - `[#] [beginner|mid|senior] [axis] file:line: problem → recommended fix (impact: H/M/L, effort: S/M/L)`
    - On the next line, a suggested commit message in the repo's style, e.g. `refactor: extract crossover detection from Macd::signal`.
  - Effort scale:
    - S = up to 5 minutes
    - M = up to 10 minutes
    - L = up to 15 minutes
    - Anything bigger must be split or moved to **Larger refactors**.
  - Sort order:
    - highest impact first
    - within the same impact, lowest effort first
    - quick wins (H/S) at the top
  - If one finding depends on another, note `after #N` and keep them in order.
  - Merge findings only when they are truly one commit. Never merge them into something bigger than a commit.
  - Keep each fix as a description, with at most a short illustrative snippet. Do not write a full implementation.
  - Mark where the 1-hour budget runs out (`— ~1h —`), so it's clear which findings fit in one sitting.
- **Test gaps to cover:**
  - Use one bullet per gap:
    - `behavior → why it matters → suggested test name`
  - If there are no gaps, say so explicitly.
- **Clippy** (real output, target only):
  - **Default lints:** each warning as `file:line: lint_name: message`, or "clean".
  - **Pedantic (advisory):** same format, if it was run. These are suggestions, not requirements.
  - Clippy warnings that are worth fixing also appear as findings above. Don't list them twice in the findings.
- **Larger refactors** (only if needed): improvements too big for a single commit that can't be split cleanly.
  - One bullet each: the problem, the direction, and why it can't be split.
  - Keep this list short. Everything that can be split belongs in the main findings as a sequence.
- **Not recommended:** things considered and rejected, with the reason (e.g. "would be over-engineering", "contradicts decision X").
- **Senior bar checklist.** Mark each ✅ or ❌:
  - [ ] no look-ahead bias, off-by-one or warm-up errors in time-series logic
  - [ ] numeric handling is sound (NaN, float comparison, money precision) and results are deterministic
  - [ ] no `unwrap`/`expect` without a justifying comment in non-test code
  - [ ] errors are typed and carry context
  - [ ] each function reads at a single level of abstraction
  - [ ] crates are used through their own abstractions, with nothing hand-rolled that they provide and no unneeded crates or features
  - [ ] no needless allocation or clone on hot paths
  - [ ] names match the domain model
  - [ ] nothing is speculative, unused or duplicated
  - [ ] the public API is documented and minimal, and follows the Rust API Guidelines
  - [ ] lifetimes and generics carry their weight (no needless explicit lifetimes, overly strict `'static` bounds, or generic bloat)
  - [ ] tests cover behavior, edge cases and error paths (no gaps)
  - [ ] logging is consistent and structured
  - [ ] clippy is clean for the target

End the report there. Do not offer to implement the findings or propose next steps.
