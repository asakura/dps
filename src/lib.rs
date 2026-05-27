#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]
#![allow(
    rustdoc::private_doc_tests,
    reason = "Module-level doc examples reference crate paths that are private to rustdoc"
)]

//! DPS — interactive terminal MOD and ppO₂ tables for nitrox dive planning.

pub mod action;
pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod environment;
pub mod errors;
pub use errors::Error;
pub mod gas;
pub mod logging;
/// Application interaction modes.
pub mod mode;
pub mod theme;
/// Terminal setup, event loop, and input event types.
pub mod tui;
pub mod ui;
pub mod units;
