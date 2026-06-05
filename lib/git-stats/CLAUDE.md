# dps-git-stats

Binary crate — all workspace code quality standards apply (see root `CLAUDE.md`).

## Additional notes

- This is a `[[bin]]` crate, not a `[lib]`. Public items that exist only for
  testability (e.g. `format_time`, `date_in_range`) are `pub` rather than
  `pub(crate)` to satisfy the `redundant_pub_crate` clippy lint.
- `std::process::exit` is denied by the workspace `exit` lint; use
  `main() -> std::process::ExitCode` instead.
- `eprintln!` / `println!` are denied; write to `anstream::stdout()` /
  `anstream::stderr()` via `writeln!`.
- The `Error` enum boxes its inner variants (`Box<GitError>` etc.) to satisfy
  the `result_large_err` clippy lint. Use `From<SubError> for Error`
  (already implemented) so `?` handles boxing automatically.
