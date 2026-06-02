//! Error types, panic hook setup, and debug utilities for DPS.

use color_eyre::Result;
use tracing::error;

use std::env;

/// Application-level error, wrapping all domain and configuration errors.
///
/// Implements [`std::error::Error`] via `thiserror`, so any variant converts
/// to [`color_eyre::Report`] through `?` in functions that return
/// [`color_eyre::Result`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Gas domain error (blend validation, membrane fractions, …).
    #[error(transparent)]
    Gas(#[from] crate::gas::GasError),
    /// Configuration error (key parsing, file loading, …).
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    /// Component error (invalid state, …).
    #[error(transparent)]
    Component(#[from] crate::components::ComponentError),
    /// Action parse error (unknown variant, malformed payload, …).
    #[error(transparent)]
    Action(#[from] crate::action::ActionError),
    /// Unit parse error (bad suffix, non-numeric value, …).
    #[error(transparent)]
    Unit(#[from] crate::units::UnitError),
    /// Key-sequence parse error (empty sequence, unknown key, …).
    #[error(transparent)]
    KeyMap(#[from] crate::keymap::KeyMapError),
    /// Register-value parse error (unknown value string, …).
    #[error(transparent)]
    Register(#[from] crate::registers::RegisterError),
    /// Application event-loop error (component failure, channel send, I/O).
    #[error(transparent)]
    App(#[from] crate::app::AppError),
    /// Logging initialisation error (I/O, bad filter directive, already initialised).
    #[error(transparent)]
    Logging(#[from] crate::logging::LoggingError),
}

/// Installs the `color_eyre` panic and error hooks.
///
/// # Errors
///
/// Returns `Err` if the `color_eyre` hook is already installed.
pub fn init() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();

    eyre_hook.install()?;

    std::panic::set_hook(Box::new(move |panic_info| {
        // process::exit bypasses Drop, so restore the terminal explicitly.
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show,
        );

        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, metadata, print_msg};

            let metadata = metadata!();
            let file_path = handle_dump(&metadata, panic_info);

            print_msg(file_path, &metadata)
                .expect("human-panic: printing error message to console failed");

            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }

        let msg = format!("{}", panic_hook.panic_report(panic_info));
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        #[expect(
            clippy::exit,
            reason = "intentional in a panic hook; terminal cleanup is performed manually above"
        )]
        std::process::exit(1);
    }));
    Ok(())
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        match $ex {
            value => {
                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                value
            }
        }
    }};
    (level: $level:expr, $ex:expr) => {
        trace_dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        trace_dbg!(level: tracing::Level::DEBUG, $ex)
    };
}
