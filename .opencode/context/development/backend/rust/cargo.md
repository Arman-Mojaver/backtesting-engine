<!-- Context: development/backend/rust/cargo | Priority: high | Version: 1.0 | Updated: 2026-05-23 -->

# Cargo — Rust Package Manager

**Version**: Rust 2021 edition (lock file present — `Cargo.lock`)
**Project**: `backtesting-engine` v0.1.0

---

## Key Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run default binary
cargo run

# Run with arguments
cargo run -- <args>

# Add a dependency
cargo add serde

# Add a dev dependency
cargo add --dev criterion

# Remove a dependency (edit Cargo.toml manually, then)
cargo update

# Update lockfile
cargo update

# Show dependency tree
cargo tree
```

---

## Cargo.toml Structure

```toml
[package]
name = "backtesting-engine"
version = "0.1.0"
edition = "2021"
default-run = "backtesting-engine"

[lib]
doctest = false

[dependencies]
actix-web = "4.11.0"
chrono = "0.4.42"
log = "0.4.28"
serde_json = "1.0.145"

[dev-dependencies]
# test-only deps go here

[profile.release]
opt-level = 3
```

---

## Workspaces (Multi-Crate)

If the project grows into multiple crates:

```toml
# root Cargo.toml
[workspace]
members = [
    "crates/engine",
    "crates/api",
]
```

Run from workspace root: `cargo build --workspace`

---

## Running Scripts / Binaries

Define multiple binaries in `Cargo.toml`:

```toml
[[bin]]
name = "backtesting-engine"
path = "src/main.rs"

[[bin]]
name = "cli"
path = "src/bin/cli.rs"
```

Run with:

```bash
cargo run --bin cli -- <args>
```

---

## Related Context

- **Testing** → `testing.md`
- **Actix-web patterns** → `actix-patterns.md`
