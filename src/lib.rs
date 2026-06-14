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

//! DPS — interactive terminal MOD and $\text{pp}\ce{O2}$ tables for nitrox dive planning.

// TODO: The following re-exports are for backward compatibility after moving
// core modules to independent crates in `lib/`. Internal references within
// this crate should eventually be updated to use `dps_units`, etc. directly.
pub use dps_environment as environment;
pub use dps_units as units;

pub mod action;
pub mod app;
/// End-to-end architecture: data-flow diagram, component lifecycle, and design decisions.
// pub mod architecture;
pub mod cli;
pub mod components;
pub mod config;
pub mod errors;
pub use errors::Error;
/// Key-handling primitives: modes, sequences, maps, and chord accumulation.
pub mod keymap;
pub mod logging;
pub mod registers;
pub mod theme;
/// Terminal setup, event loop, and input event types.
pub mod tui;
pub mod ui;
