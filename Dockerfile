# Build targets:
#   fetcher  - pre-fetches crate dependencies into the cargo cache.
#   dev      - toolchain-only dev container used by docker-compose. It contains
#              no engine binary; compose mounts the source and you build inside
#              it (`bte` symlinks to /app/target/debug/bte).
#              Build with: docker build --target dev .
#   builder  - release build of `bte` for aarch64 (linux/arm64 — the only
#              architecture this image ships for; it runs on Apple Silicon).
#   runtime  - default (last) target: non-root Debian image with only the
#              release binary, matching the dev container's userland (bash,
#              coreutils, ca-certificates) minus the toolchain. No ENTRYPOINT —
#              it is a long-running workspace container (CMD sleep infinity):
#              start it, shell in, call `bte` from within.
#
# Debian release: rust:1.98.1 is trixie (glibc 2.41). The builder and runtime
# pin trixie explicitly so the binary's glibc always matches the runtime's.

FROM rust:1.98.1 AS fetcher
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "" > src/lib.rs
RUN echo "fn main() {}" > src/bin/bte.rs
RUN cargo fetch


FROM rust:1.98.1 AS dev
RUN apt-get update && apt-get install -y \
   ca-certificates \
   build-essential \
   mold \
 && rm -rf /var/lib/apt/lists/*

COPY --from=fetcher /usr/local/cargo /usr/local/cargo
RUN ln -sf /app/target/debug/bte /usr/local/bin/bte

WORKDIR /usr/src/app

CMD ["sleep", "infinity"]


FROM rust:1.98.1-trixie AS builder
WORKDIR /app

# mold: release links are the slow tail under lto + codegen-units=1.
RUN apt-get update && apt-get install -y --no-install-recommends mold \
 && ln -sf /usr/bin/mold /usr/bin/ld.mold \
 && rm -rf /var/lib/apt/lists/*

# M2 tuning (see README, Building and installation), done CI-safely: `-C target-cpu=native` is only correct
# on the Mac itself — on GitHub's arm64 runners it would tune for the runner's
# Neoverse cores, and it is invalid when cross-compiling. `apple-m2` is the
# explicit target (rustc 1.98.1 honors it; its feature set is identical to
# `apple-a15` — the same Avalanche microarchitecture — if a future LLVM ever
# drops the alias). The per-target form keeps host build scripts unflagged, and
# mold only affects the final link of target units.
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C target-cpu=apple-m2 -C link-arg=-fuse-ld=mold"

COPY --from=fetcher /usr/local/cargo/registry /usr/local/cargo/registry

# Dependency layer: fake sources so every lockfile dependency compiles and is
# cached as its own image layer. Only Cargo.toml/Cargo.lock changes invalidate
# it — source edits rebuild just the workspace crate.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
 && echo "" > src/lib.rs \
 && echo "fn main() {}" > src/bin/bte.rs \
 && cargo build --release --locked --target aarch64-unknown-linux-gnu --bin bte

# Real sources. `touch` is required: the context files' mtimes can predate the
# dependency artifacts above, which cargo would otherwise read as "fresh".
# `strip --strip-debug`: on Linux the line tables embed in the ELF (~40 MB
# here — macOS keeps them outside the binary via split-debuginfo), and the
# shipped image does not need them; the symbol table stays, so backtraces
# keep function names.
COPY src ./src
RUN touch src/lib.rs src/bin/bte.rs \
 && cargo build --release --locked --target aarch64-unknown-linux-gnu --bin bte \
 && strip --strip-debug target/aarch64-unknown-linux-gnu/release/bte


FROM debian:trixie-slim AS runtime
# Mirrors the dev container's userland (Debian trixie: bash, coreutils,
# ca-certificates) minus the toolchain — same get-in-and-call-the-CLI workflow,
# release binary instead of the debug build. Same Debian release as the
# builder, so the binary's glibc (2.41) matches exactly.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /app --shell /usr/sbin/nologin bte \
 && mkdir -p /app/logs /app/parquet_files \
 && chown -R bte:bte /app

COPY --from=builder /app/target/aarch64-unknown-linux-gnu/release/bte /usr/local/bin/bte

USER bte
WORKDIR /app

CMD ["sleep", "infinity"]
