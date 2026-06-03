# dps-environment API Contract & Standards

This document defines the architectural and documentation standards for
the `dps-environment` crate.

## Robustness & Invariants

- **Strong Typing**: Use the `dps-units` newtype system (`Bar`, `Meters`,
  `Celsius`, `MetersPerBar`) for all public signatures to prevent unit mix-ups.
- **Border Validation**: All fallible constructors (e.g., `DiveEnvironment::new`,
  `at_altitude`) must validate inputs for finiteness and positivity.
- **Immutability**: `DiveEnvironment` is `Copy` and immutable. State transitions
  occur via builder methods returning a new `Result<Self>`.

## API Discoverability & Stability

- **Preset Enums**: Mark `Ocean` and `Lake` enums as `#[non_exhaustive]` if they
  are intended to grow, preventing breaking changes in downstream `match`
  statements.
- **Public Limits**: Export physical validation limits (e.g., `MAX_ALTITUDE`,
  `MAX_SALINITY_PPT`) so consumers can perform pre-validation or set UI bounds.

## Serialization Contract

- **Symmetrical Roundtrips**: `Display` and `FromStr` must be perfect inverses.
- **Format Support**: Support both named presets (`ocean:RedSea`) and custom
  values (`surface_pressure=P,water_density=D`) for clipboard-safe serialization.
- **Serde Support**: Provide optional `serde` implementations for all public
  data structures.

## Documentation Standards

- **KaTeX Integration**: Use KaTeX for all physical formulas in doc comments as
  specified in the root `CLAUDE.md`.
- **Mathematical Transparency**: Explicitly document underlying physical models
  (e.g., ICAO ISA, linear density approximation).
- **Doc Tests**: Every public method must have a doc test demonstrating correct
  usage and physical expectations.

## Validation & Testing

- **Integration Tests**: Maintain `tests/` to verify the crate's public API from
  an external consumer's perspective.
- **Roundtrip Testing**: Exhaustively test `Display` and `FromStr` roundtrips
  to ensure zero data loss.
- **Physical Correctness**: Verify that presets (e.g., `Ocean::RedSea`) align
  with established physical constants.
