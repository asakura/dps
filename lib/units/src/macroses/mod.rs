//! Internal macros for generating newtype unit implementations.
//!
//! ```
//! use dps_units::Bar;
//! let v = Bar::new(1.0);
//! assert_eq!(v.to_string(), "1.0 bar");
//! ```

mod impl_percent_scaling;
mod unit_newtype;
mod unit_newtype_common;
