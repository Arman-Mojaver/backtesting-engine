#![cfg_attr(
    test,
    expect(
        clippy::unreadable_literal,
        reason = "test data keeps Unix timestamps and captured EURUSD values as copied from their source"
    )
)]

pub mod application;
pub mod config;
pub mod domain;
pub mod engine;
pub mod infrastructure;
