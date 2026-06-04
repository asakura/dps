# dps-gas: Gas Mix and Blending API Contract

This document defines the architectural and documentation standards for
the `dps-gas` crate, which models Enriched Air Nitrox (EANx) blends and
provides depth-safety calculations for dive planning.

## Design Principles

- **Strong Typing**: Use `Percent`, `Bar`, `Meters`, etc. from `dps-units` for
  all public signatures. Never accept or return raw `f64` for physical quantities.
- **Invariant Enforcement at Construction**: Both `EANxBlend::new` and
  `Membrane::from_analysis` reject invalid inputs eagerly. Once constructed, an
  `EANxBlend` carries the guarantee that `fo2 >= 10%` and that the blend method's
  ceiling is not exceeded. Do not add fallible methods that could silently violate
  these invariants.
- **Sealed `BlendMethod`**: The trait is sealed via a private `sealed::Sealed`
  supertrait. Do not make it implementable externally — the blend methods encode
  physical models that must be audited together.
- **Environment-Aware Calculations**: All depth-related methods (`ppo2_at`,
  `mod_at`, `end_at`, etc.) read `self.env` for surface pressure and water density.
  Default to `DiveEnvironment::standard()` at construction; let callers override
  via `.with_environment()`.
- **Lossless Serialization**: All public types support `serde` (via the `serde`
  feature). Deserialization re-validates invariants using the shadow-type pattern.

## API Standards

- **Errors**: The top-level error is `GasError` (alias for `error::Error`), which
  wraps `InvalidEANxError` and `InvalidMembraneFractionsError` via `#[from]`. Each
  module owns its own error enum; callers import `GasError`.
- **Traits**: Public types implement `Debug`, `Clone`, `Copy`, `PartialEq` where
  physically meaningful. `Eq` only where exact equality makes sense (avoid for
  `f64`-backed types without special handling).
- **`Default`**: `EANx::default()` returns air (21 % O₂). `Membrane::default()`
  returns `Membrane::typical()`. Provide `Default` only when a physically
  meaningful canonical value exists.
- **Naming convention**: `EANx` display names follow the NOAA/dive-industry
  standard: `"Air"`, `"EANx 32"`, `"Hypoxic 10"`, `"O₂ 50%"`, `"Pure O₂"`.
  Non-standard fractions fall back to `"N%"` from `Percent::Display`.
- **Depth calculations return summary types**: `mod_at` returns `MOD`, `end_at`
  returns `END`, etc. Summary types carry the inputs alongside the result so
  callers can display context without re-computing.
- **`best_mix`**: Only available on `EANxBlend<PartialPressure>` (i.e. `EANx`).
  PSA and membrane best-mix calculations require separate treatment not yet modelled.
- **Gas density**: Computed via the ideal gas law at the ISO 20 °C reference
  temperature. Reports `GramsPerLitre`. The density threshold for elevated work
  of breathing risk is ≈ 5.7 g/L (note this in any UI that surfaces this value).

## Serialization Contract

- **Shadow-type deserialization**: `EANxBlend`, `GasComponents`, and `Membrane`
  each use a private `*Shadow` struct that `serde` deserializes into, then
  `TryFrom` re-validates the invariants. Never deserialize directly into the
  real struct fields — doing so bypasses construction-time validation.
- **`GasComponents` deserialization**: Requires `|sum − 1.0| ≤ 1e-6`. Reject
  anything looser; the tolerance comes from IEEE 754 accumulation, not from
  physics.
- **`Membrane` deserialization**: Requires diluent ratios (fn2 + far + fco2 +
  fother) to sum to 1.0 ± 1e-6.
- **No clipboard roundtrip for `EANxBlend`**: Unlike `dps-units` and
  `dps-environment`, gas blends do not define `to_clipboard_string()` / `FromStr`
  for the generic `EANxBlend<M>`. Only `EANx` (partial-pressure) implements
  `FromStr`. When adding a new serialisation format, follow this asymmetry and
  document it clearly.

## Documentation Standards

- **KaTeX Integration**: Use `$\pu{value unit}$` (mhchem) for physical quantities
  in doc comments, as specified in the root `CLAUDE.md`. Escape backslash-
  punctuation combinations as documented there (e.g. `$\\,$` for a thin space).
- **Formula documentation**: Every depth-calculation method (`ppo2_at`, `mod_at`,
  `ead_at`, `end_at`, etc.) must show the governing formula in its doc comment.
  Example format: `Formula: ppO₂ = (depth / ρ + P_surface) × FO₂`.
- **Physical model attribution**: Name the source model explicitly (e.g.
  "NOAA narcosis model", "NOAA single-dive CNS exposure limits", "ICAO ISA").
  This helps reviewers validate constants and catches model drift in the future.
- **Doc Tests**: Every public method must have at least one runnable doc test.
  Use `no_run` only when the example requires external state (e.g. a
  `DiveEnvironment` preset unavailable in the doctest harness); prefer runnable
  examples.

## Validation & Testing

- **Colocated tests**: `#[cfg(test)] mod tests { … }` inside the same file.
  Integration tests that exercise the public API from outside the crate go in
  `tests/`.
- **Submodule naming**: Group tests in submodules named after the item under test
  (`mod mod_at { … }`, `mod display { … }`), in source order.
- **`approx` for float comparisons**: Use `assert_relative_eq!` and
  `assert_abs_diff_eq!` from the `approx` crate. Never compare `f64` or
  `GramsPerLitre` / `Bar` etc. with `==` in tests, except where exact equality
  is guaranteed (e.g. `OTUPerMinute::new(0.0)` below threshold).
- **Physical correctness**: Test that air's narcotic fraction ≈ 0.7948, molar
  mass ≈ 28.97 g/mol, and surface density ≈ 1.204 g/L. These are the anchor
  values that detect model drift.
- **Cross-method invariants**: Assert that PSA and membrane blends produce more
  Ar than the partial-pressure equivalent at the same FO₂. Assert that PSA is
  denser than PP at the same FO₂ (higher Ar molecular weight).
- **Boundary conditions**: Test the 10 % O₂ floor, the PSA ceiling, and the
  CNS table boundaries (0.50, 0.60, …, 1.60 bar) as distinct test cases.
