# backtesting-engine

[![CI](https://github.com/Arman-Mojaver/backtesting-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/Arman-Mojaver/backtesting-engine/actions/workflows/ci.yml)

A Rust engine that generates and evaluates trading strategies against historical daily (D1)
market data. It loads indicators, OHLCV candles, and long/short trade records from
PostgreSQL, enforces five data-integrity invariants before computing anything, derives
entry signals from real indicator mathematics, scores each candidate strategy on a
single-pass compounding curve, and persists the survivors to partitioned, idempotent
Parquet output.

**Status: pre-1.0, under active development.** The pipeline is real end-to-end, but the
metric model has a known simplification (see [Known limitations](#known-limitations))
and the HTTP API is deliberately absent. Read that section before drawing conclusions
from the output.

## Table of contents

- [What it does](#what-it-does)
- [Architecture](#architecture)
- [The strategy pipeline](#the-strategy-pipeline)
- [Domain model](#domain-model)
- [Validation invariants](#validation-invariants)
- [Indicators and signal semantics](#indicators-and-signal-semantics)
- [Performance metrics](#performance-metrics)
- [Strategy filtering](#strategy-filtering)
- [CLI reference](#cli-reference)
- [Output format](#output-format)
- [Configuration](#configuration)
- [Building and installation](#building-and-installation)
- [Development](#development)
- [Known limitations](#known-limitations)
- [Ecosystem](#ecosystem)
- [License](#license)

## What it does

A backtest run is scoped to one **cell**: a single `(instrument, money management
strategy)` pair. For each cell, the engine:

1. **Fetches** indicators, daily candles, and long/short trade records from PostgreSQL.
2. **Validates** the data against five fail-fast invariants (non-emptiness, same
   instrument, same money management strategy, timestamp alignment, chronological
   resampled points). Bad data never reaches the compute path.
3. **Computes** indicator values over the candle series and derives **signal
   timestamps** from them — zero-crossings, threshold crossings, or line crossovers,
   depending on the indicator kind.
4. **Builds one candidate strategy per indicator**, resolving each signal to the
   corresponding trades and scoring it — global ROI, max draw down, annualized ROI,
   annual operation count — in a single pass over a compounding curve.
5. **Filters** in two stages — cheap first (activity), expensive second (draw down +
   ROI) — and **persists** survivors to Parquet, partitioned by instrument and money
   management strategy.
6. **Records completed work** in a processed manifest, making every command
   **idempotent**: re-running skips work already done.

`bte strategy create_loop` runs the same pipeline over *all* cells in the
database: each source table is loaded once, cells are computed in parallel with rayon,
and output is committed as a single batch.

## Architecture

Ports-and-adapters (hexagonal) with four layers. Dependencies point inward only — the
domain layer has no I/O and no framework types, so the business rules can be understood
and unit-tested in complete isolation.

```
CLI              bte: the native Rust engine — runs directly, no operator frontend

Application      use-case orchestration: fetching inputs, generating signals,
                 building and filtering candidate strategies

Domain           pure business logic: entities, validation invariants,
                 indicator mathematics, signal semantics, performance metrics

Infrastructure   adapters: PostgreSQL reads, partitioned Parquet writes,
                 the processed manifest, logging
```

The split is deliberate: business rules live in exactly one place and are tested
without a database; everything that touches the outside world sits behind an adapter
boundary and can be swapped without touching the rules.

## The strategy pipeline

```
fetch (PostgreSQL) → diff vs processed manifest → validate invariants
→ derive signals → drop inactive signal sets → build candidate strategies
→ apply quality bar → persist (Parquet) → record in manifest
```

Stage notes:

- **Idempotency first.** Before any computation, the engine diffs the database's
  indicators against the processed manifest and computes only the unprocessed
  remainder. A fully-processed cell is skipped without touching the compute path.
- **Validation is fail-fast and ordered.** The five invariants run in a fixed order and
  the first failure aborts the cell. Emptiness is checked first because the cross-dataset
  checks pass vacuously on empty input.
- **Signals are real.** Each indicator's values are computed from the candle series and
  converted to entry timestamps by the indicator's signal rule (see
  [Indicators and signal semantics](#indicators-and-signal-semantics)).
- **Two-stage filtering.** Signal sets below the activity bar are dropped *before* any
  strategy is built; the draw-down/ROI bar runs afterwards on the (smaller) candidate
  set. Cheapest test first, always.
- **Batch runs.** `create_loop` loads each source table once (queries run in parallel),
  computes every cell in parallel, assigns strategy IDs in one sequence, and commits
  the manifest in a single atomic write.

## Domain model

| Concept | Meaning |
|---|---|
| **Instrument** | A tradeable pair, e.g. `EURUSD`. |
| **Operation point** | One trade entry: outcome (win/loss), take-profit, stop-loss, risk fraction, governing MMS, timestamp. Long and short operation points are kept in **separate collections** — direction is where a record lives, not a field on it. |
| **Resampled point** | A daily OHLCV candle plus the instrument it belongs to. |
| **Indicator** | A typed indicator configuration: kind plus parameters (periods, thresholds). |
| **Signal** | A timestamp at which an indicator calls for a long or short entry. |
| **Signal group** | All long and short signals for one indicator. |
| **Strategy** | One indicator, the long/short trades it selects, its MMS, and its performance metrics. |
| **Money management strategy (MMS)** | The position-sizing and risk rule governing trades, referenced by ID. |
| **Cell** | One `(instrument, MMS)` pair — the unit of work and of output partitioning. |

All timestamps are epoch seconds.

## Validation invariants

Five invariants, checked in order, failing fast:

| # | Invariant | Rule |
|---|---|---|
| 1 | Non-empty | All four datasets (indicators, candles, long trades, short trades) are non-empty. |
| 2 | Same instrument | Every candle and trade references the same instrument. |
| 3 | Same MMS | Every trade's money management strategy matches the run's expected MMS. |
| 4 | Timestamp alignment | Long and short trade timestamps are identical sets, and both are contained in the candle timestamps. |
| 5 | Chronological resampled points | Resampled point timestamps are strictly increasing: an unsorted series or a repeated timestamp is rejected, never repaired. |

Violations are reported as typed errors carrying the specific invariant that failed.

## Indicators and signal semantics

Each indicator computes a timestamped value series over the candles. The series kind
determines the signal rule:

| Series kind | Signal rule | Long | Short |
|---|---|---|---|
| Oscillator | Zero-crossing | value crosses − → + | value crosses + → − |
| Threshold | Threshold crossing | crosses **up** through the low threshold | crosses **down** through the high threshold |
| Crossover | Difference of two lines, then zero-crossing | diff crosses − → + | diff crosses + → − |

An operation point stamped T is entered at the open of resampled point T, but an
indicator value at T is only known at T's close. So a cross read at T is stored as a
signal on the **next** resampled point (the next timestamp in the series, not T plus one
day), and a cross on the last resampled point is dropped. Strategies match each signal to
the operation point with the same timestamp, so they never trade on data from the future.

## Performance metrics

All metrics are pure functions of the trade sequence — no I/O, unit-tested in
isolation. Both curve metrics are computed in a **single pass** over one compounding
curve, processed in chronological order of entry regardless of the order the trades
arrive in:

```
contributionᵢ = cummaxᵢ₋₁ · resultᵢ · (riskᵢ / slᵢ)
cumsumᵢ       = cumsumᵢ₋₁ + contributionᵢ        (cumsum₀ = 1.0)
cummaxᵢ       = max(cummaxᵢ₋₁, cumsumᵢ)          (cummax₀ = 1.0)

global_roi     = (cumsumₙ − 1) · 100
max_draw_down  = maxᵢ (cummaxᵢ − cumsumᵢ) / cummaxᵢ · 100
```

Both are rounded to 2 decimal places independently. Derived metrics:

```
years                    = (end_ts − start_ts) / 86400 / 365.25
annual_roi               = ((1 + global_roi/100)^(1/years) − 1) · 100
annual_operation_count   = operation_count / years
```

Zero counts and non-positive time spans return `0.0` rather than dividing by zero. A total
loss (accumulated value ≤ 0) gives an annual ROI of `0.0`, never NaN.

## Strategy filtering

Two stages, cheapest first:

1. **Activity stage** — signal sets under the minimum annual operation count are
   dropped *before* any strategy is built.
2. **Quality stage** — candidate strategies must meet the draw-down and ROI bars.

The default bars (a draw-down ceiling, a minimum annual ROI, and a minimum annual
operation count) are defined in code, as `GlobalStrategyFilter::default()`, and can be
overridden per call. They are not repeated here, so they are maintained in one place.

## CLI reference

One binary: `bte`.

### `bte` — native Rust engine

Runs inside the engine container (or anywhere it can reach PostgreSQL and the Parquet
directory).

```
bte strategy create -m <MMS_ID> [-i <INSTRUMENT>]   # one cell (default instrument: EURUSD)
bte strategy create_loop                           # all cells, idempotent batch
```

## Output format

Strategies are written as Parquet (ZSTD-compressed) under an environment-scoped
directory, partitioned to mirror the domain's unit of work:

```
parquet_files/{environment}/strategy/
└── instrument={instrument}/
    └── money_management_strategy_id={mms_id}/
        └── strategies.parquet
```

Column layout (the data contract for downstream readers):

| Column | Type |
|---|---|
| `strategy_id` | `Int64` |
| `indicator_id` | `Int32` |
| `annual_roi` | `Decimal128(10, 2)` |
| `max_draw_down` | `Decimal128(10, 2)` |
| `annual_operation_count` | `Decimal128(10, 2)` |
| `long_operation_point_ids` | `List<Int32>` |
| `short_operation_point_ids` | `List<Int32>` |

Write semantics:

- **Strategy IDs are assigned at persistence time** — globally unique across all
  partitions, contiguous within a batch. The domain never mints IDs.
- **Partitions are idempotent** — re-running a cell never duplicates rows for an
  indicator already present in the partition.
- Parquet files are immutable: appending means read, merge, rewrite.

### Processed manifest

A single Parquet ledger next to the partitions records every completed
`(instrument, MMS, indicator)` triple with a completion timestamp. It is the engine's
idempotency ledger: `create` and `create_loop` compute only what it doesn't list,
appends are deduplicated on the triple, and writes are atomic (temp file + rename).
It does not track the engine version, the filter thresholds or the source data, so a
cell it lists can hold stale strategies (see
[Known limitations](#known-limitations), item 8).

## Configuration

| Variable | Purpose | Example |
|---|---|---|
| `ENVIRONMENT` | Selects DB target and Parquet subdirectory | `development` |
| `PARQUET_FILES_PATH` | Host path mounted at the container's Parquet root | `./parquet_files` |
| `RUST_LOG` | Log level | `info` |

Database targets are fixed per environment (Docker-oriented — the host is always
`host.docker.internal`):

| `ENVIRONMENT` | Port | Database |
|---|---|---|
| `production` | 5432 | `db-production` |
| `development` | 54320 | `db-development` |
| `testing` | 54321 | `db-testing` |
| `alpha` | 54322 | `db-alpha` |
| `beta` | 54323 | `db-beta` |

User/password are `postgres`/`postgres`. There is currently **no env override for DB
credentials** — see [Known limitations](#known-limitations).

## Building and installation

Requires Rust — the exact version is pinned in `rust-toolchain.toml` (rustup selects it automatically; CI and the Docker image use the same pin).

```bash
cargo build                                # debug
cargo build --release

# Install the binaries to ~/.cargo/bin
cargo install --path . --bin bte --force
```

The Docker runtime image bakes `-C target-cpu=apple-m2` into its release build
(see Dockerfile) — explicit rather than `native`, because CI builds on non-Apple
arm64 runners where `native` would tune for the wrong CPU. For bare
`cargo build --release` on the Mac itself, `RUSTFLAGS="-C target-cpu=native"` is
equivalent (the host is the M2) and worth ~free performance.

### Docker workflow

```bash
cp .env.example .env        # ENVIRONMENT, PARQUET_FILES_PATH, RUST_LOG

make up                     # start the dev container
make in                     # shell into it — bte is on PATH
make logs                   # follow container logs
make build                  # build dev image (multi-stage, dep-compile cache layer)
make fmt                    # cargo fmt
```

`docker-compose.yaml` runs the **dev** image: it mounts the source, the Parquet
directory, and the logs, with named volumes for the cargo registries and build
artifacts — you compile inside the container.

#### Running the runtime image (linux/arm64)

The published runtime image (`armanmojaver/backtesting-engine:latest`) is built
by CI on GitHub's arm64 runners and targets Apple Silicon only. It mirrors the
dev container's userland (Debian trixie: bash, coreutils, ca-certificates) minus
the toolchain, with the release binary at `/usr/local/bin/bte`. The workflow is
identical to dev — get in and call the CLI:

```bash
make pullr                  # fetch the CI-built image (or buildr to build locally)
make upr                    # start the long-running runtime container
make inr                    # bash inside it — bte is on PATH
# inside the container:
bte strategy create_loop
make downr                  # stop it
```

One-shot runs without entering the container:

```bash
docker compose -f docker-compose.runtime.yaml run --rm engine bte strategy create
```

The image has no ENTRYPOINT — commands are always explicit, so nothing can be
misread as a `bte` subcommand (the `error: unrecognized subcommand 'sh'` failure
came from an ENTRYPOINT design that is now gone).

## Development

```bash
cargo fmt --check                                          # formatting (CI-enforced)
cargo clippy --all-targets --all-features -- -D warnings   # zero-warning policy (CI-enforced)
cargo test                                                 # no PostgreSQL or Docker required
```

The test suite is unit tests plus file-based adapter tests under temp directories —
it runs with no database and no containers. CI runs lint (formatting, clippy, rustdoc),
tests, and cargo-deny on every push to `main` (toolchain pinned to match the Docker
image), then builds and pushes
linux/arm64 images on GitHub's arm64 runners: `latest` + short commit SHA (runtime)
and `dev`. The images target Apple Silicon only.

## Known limitations

Honest list — read before trusting the numbers:

1. **Metrics assume trades close in entry order.** The compounding curve walks trades
   sorted by entry timestamp. Real trades have exit dates, and positions do not
   necessarily close in the order they were opened. Accounting for exit dates is a
   required change before any production use of ROI/draw-down figures.
2. **Data management and strategy display live in the companion Python service.** The
   Rust engine is `bte`-only — all data CRUD and the iteration table are owned
   by [Backtesting-4.0](../Backtesting-4.0) and accessed there directly.
3. **DB configuration is hardcoded per environment** (host, ports, credentials). Fine
   for the Docker workflow; limiting if you want to point the engine at arbitrary
   PostgreSQL instances.
4. **No HTTP API.** Deliberate (YAGNI) — CLI only until real API consumers exist.
5. **One instrument and one MMS per run.** Enforced by validation —
   this is a domain constraint, not an accident, but it shapes how you batch work.
6. **Parquet write cost.** Appending to a partition rewrites the whole file, and
   strategy ID assignment scans all partitions for the global max. Both are fine at
   current scale and will need rethinking if partition counts grow by orders of
   magnitude.
7. **Command handlers are thin wrappers over tested logic; the wrappers themselves
   have no direct tests.**
8. **Stored strategies are not recomputed when their inputs change.** The processed
   manifest records only which `(instrument, MMS, indicator)` triples are done, not how
   or from what. After any of these changes, the stored strategies are stale and every
   re-run still skips them:
   - the engine's logic: signal semantics, indicator math, metric formulas, validation
     rules;
   - the default filter bars;
   - the source data in PostgreSQL for a cell: new or corrected resampled points or
     operation points, or an indicator whose parameters were edited under the same id.

   To recompute, delete the whole environment directory of the
   [output layout](#output-format), `parquet_files/{environment}/` (on the host, under
   `PARQUET_FILES_PATH`), then run `bte strategy create_loop`. That removes the strategy
   partitions and the processed manifest together. A corrupt partition is recovered the
   same way. Consequences:
   - Strategy IDs start again at 1, so an ID stored outside the engine no longer points
     at the same strategy.
   - A reset is per environment: resetting `development` never touches `production`. A
     single cell cannot be reset, because one manifest covers every cell.

   Never delete half of the tree:
   - **Only the manifest:** the cells are recomputed, but each partition keeps the row
     it already holds for an indicator and drops the fresh one. The old strategies
     survive, including ones that no longer pass the filter.
   - **Only the partitions, or one corrupt partition:** the manifest still lists their
     indicators as processed, so nothing is recomputed and those strategies are lost.

## Ecosystem

This engine is the Rust computation core of a two-repo system. The companion
**[Backtesting-4.0](../Backtesting-4.0)** repository (Python/Django) owns persistence,
user-facing APIs, data CRUD, and the iteration table (strategy display). The Rust
engine is the standalone `bte` binary and has no dependency on the Python
service.

## License

[MIT](LICENSE) © Arman-Mojaver
