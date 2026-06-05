//! `dps-git-stats` — per-day git commit statistics.
//!
//! Walks the commit graph with [`gix`], aggregates with [`polars`], and
//! renders a coloured table plus an hour distribution chart to stdout via
//! [`anstream`].
//!
//! # Quick start
//!
//! ```rust
//! // Parse the CLI args without actually running (no real repo needed).
//! use clap::Parser;
//! use dps_git_stats::args::Args;
//!
//! let args = Args::try_parse_from(["dps-git-stats", "--since", "2024-01-01"]).unwrap();
//! assert_eq!(args.since.as_deref(), Some("2024-01-01"));
//! ```

#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

mod args;
mod error;
mod git;
mod render;
mod stats;

use std::io::Write as _;
use std::process::ExitCode;

use clap::Parser;

use crate::args::Args;
use crate::error::Error;
use crate::render::Renderer;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let mut stderr = anstream::stderr();
            let _ = writeln!(stderr, "error: {e}");

            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Error> {
    let args = Args::parse();

    let commit_stats = git::collect_stats(&args)?;
    let df = stats::build_frame(&commit_stats)?;
    let dist = stats::hour_distribution(&commit_stats);

    let renderer = Renderer::new(args.flavour);
    let mut out = anstream::stdout();

    renderer.render(&mut out, &df, &dist)?;

    Ok(())
}
