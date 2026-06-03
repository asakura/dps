# Code quality standards

## Documentation

- Every module (`mod.rs` or named file) must have a `//!` doc comment with
  at least one runnable `/// ``` ... ```` example.
- Every public function, method, and trait impl must have a doc comment with
  an `# Examples` section containing a runnable example.

## Tests

- Tests are colocated (`#[cfg(test)] mod tests { ... }` in the same file).
- Every test function uses `#[rstest]`, even single-case tests with no
  parameters.
- Nest tests in submodules named after the item under test
  (`mod display { ... }`, `mod from_str { ... }`), in source order.
- Within a submodule: happy path first, then error/edge cases.
- No duplicate tests: don't re-assert behaviour already covered by a
  lower-level unit.
- `Option` → `Result` in tests: `opt.ok_or_else(|| eyre!("..."))` with `?`.
  No `.unwrap()` on `Option`.
- `Result` in tests: use `?` directly. For `()` error types, return
  `Result<()>` (not the `color_eyre::Result` alias). Never `.ok()` a
  `Result`.

## Error handling

- No `.unwrap()`, `.expect()`, or panicking variants in library code —
  propagate with `?`.
- Every fallible public API returns `Result<T, E>` with a concrete error type
  (not `Box<dyn Error>` or `anyhow::Error`).
- Error types use `thiserror::Error`. Each module boundary owns one error enum
  consolidating its errors (see `lib/units/src/error.rs`, `src/keymap/error.rs`).
- Errors form a tree: module-level enums wrap lower-level errors via `#[from]`
  or `map_err`, so each layer imports one type.
- `.unwrap()` is allowed only in doc examples (`/// ``` ... ````) where panic
is the right signal. Unit tests (`#[cfg(test)]`) must use
`?`or explicit
`assert!(result.is_ok())`/`assert_eq!(result, Err(...))`— never `.unwrap()`.

## `mod` and `use` organisation

**Module declarations** (`mod foo;`, `pub mod foo;`) come first, before all
`use` statements.

**Re-exports** (`pub use self::…`) follow module declarations, before private
imports.

**Import groups** — separate each group with one blank line, in this order:

1. `use super::…` — parent-module items
2. `use crate::…` — crate-internal items
3. Third-party crates
4. `use std::…` — standard library

**Compaction** — multiple `use` statements sharing the same leading path are
merged into one braced form:

```rust
// good
use super::{ActionError, error::ParseError};
use std::{fmt, str::FromStr};

// bad — unmerged
use super::ActionError;
use super::error::ParseError;
```

Use `self::` on every `pub use` re-export (`pub use self::foo::Bar`).

**Inside `mod tests`** — follow the same four-group order. Place group-level
imports at the top of `mod tests` rather than repeating them in every
submodule. Each submodule only re-imports via `use super::*;`.
