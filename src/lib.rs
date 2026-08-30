//! Backtesting engine.
//!
//! Layered architecture: [`domain`] (pure business logic and repository
//! ports), [`application`] (use-case orchestration), [`infrastructure`]
//! (PostgreSQL and parquet adapters), [`engine`] (the `bt-engine` binary's
//! command layer) and [`cli`] (the `ct` wrapper binary).
//!
//! # Error-handling policy
//!
//! - **Environmental failures** — database I/O, decoding stored data, file
//!   I/O — are reported through `Result` types and handled at the
//!   application boundary. Repository ports return
//!   `domain::repository_error::RepositoryError`; application and engine
//!   layers propagate with `anyhow`.
//! - **Panics** are reserved for *proven invariants*: states guaranteed by
//!   the type system or by a preceding validation step (clap-required
//!   subcommands, non-empty iterator windows). Every panic site carries a
//!   message or comment justifying the invariant.
//! - **Insufficient input data** in the domain is a typed error
//!   (`domain::indicators::IndicatorError`), never a panic.

pub mod application;
pub mod cli;
pub mod config;
pub mod domain;
pub mod engine;
pub mod infrastructure;
