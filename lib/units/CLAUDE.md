# dps-units API Contract & Standards

This document defines the architectural and documentation standards for
the `dps-units` crate, which provides the type-safe foundation for all physical
calculations in the DPS project.

## Architectural Principles

- **Newtype Wrappers**: All physical quantities must be represented as
  `f64` newtypes to prevent unit mix-ups and provide type-safe arithmetic.
- **Macro-Driven Consistency**: Use the `unit_newtype!` and `unit_newtype_common!`
  macros to generate standard trait implementations (`Display`, `FromStr`, `Add`,
  `Sub`, `Mul<f64>`, etc.). This ensures a uniform API across all units.
- **Controlled Precision**:
  - `Display` must provide a human-readable representation
    (typically 1 decimal place).
  - `to_clipboard_string()` must provide a lossless representation (using `ryu`)
    to ensure bit-perfect roundtrips.
- **Zero-Cost Abstractions**: Unit wrappers must be `#[repr(transparent)]`
  (via macros) and `Copy` to ensure they have the same runtime performance as
  raw `f64`.

## Arithmetic & Safety

- **Homogeneous Operations**: Addition and subtraction are only permitted between
  values of the same unit type.
- **Dimensionless Ratios**: Dividing two values of the same unit type must return
  a unitless `f64`.
- **Scaling**: All units must support multiplication and division by `f64` scalars.
- **Explicit Inter-unit Interactions**: Operations between different units
  (e.g., `Meters / MetersPerBar -> Bar`) must be explicitly implemented to model
  physical laws.
- **Finiteness Guards**: Units should provide `is_finite()` and `is_positive_finite()`
  helpers to facilitate boundary validation in higher-level crates.

## Documentation Standards

- **Units in Docs**: Always use KaTeX for physical quantities in doc comments
  (e.g., `$\pu{1.0 bar}$`).
- **Interactive Examples**: Every unit must include doc examples demonstrating
  its primary arithmetic and serialization behavior.
- **Mathematical Rationale**: Derived units (e.g., `OTUPerMinute`) should
  document their physical meaning and the formulas they participate in.

## Serialization Contract

- **Roundtrip Guarantee**: Every unit must roundtrip perfectly from
  `to_clipboard_string()` through `FromStr`.
- **Suffix Requirements**: `FromStr` must require the correct unit suffix
  (e.g., `" m"`, `" bar"`) to prevent accidental parsing of the wrong unit type.
- **Optional Serde**: All units must support `serde` (via the `serde` feature)
  with transparent serialization of the underlying `f64`.

## Testing & Validation

- **Macro-Generated Tests**: Ensure every unit passes the standard suite of
  generated tests for arithmetic, range validation (if bounded), and serialization.
- **Approximate Equality**: Use `approx` (`RelativeEq`, `AbsDiffEq`) for all
  floating-point comparisons in tests.
- **Boundary Conditions**: Units with physical bounds (like `Percent` or
  `Celsius` in some contexts) must be tested against their limits.
