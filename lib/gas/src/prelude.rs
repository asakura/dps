//! Re-exports the full public API of `dps_gas`.
//!
//! Import with `use dps_gas::prelude::*;` to bring every public type into scope
//! without depending on the internal module layout.
//!
//! # Example
//!
//! ```
//! use dps_gas::prelude::*;
//! use dps_units::Percent;
//!
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//! assert_eq!(ean32.to_string(), "EANx 32");
//! ```

pub use crate::blend::{
    BlendMethod, InvalidMembraneFractionsError, Membrane, PartialPressure, Psa,
};
pub use crate::components::GasComponents;
pub use crate::constants::{
    AIR_AR, AIR_CO2, AIR_DILUENT, AIR_N2, AIR_NARCOTIC, AIR_O2, AIR_OTHER, EAN_MIN_O2,
};
pub use crate::eanx::{
    EAD, EADSummary, EANx, EANxBlend, EANxDetail, END, ENDSummary, InvalidEANxError, MND,
    MNDSummary, MOD, MODSummary, MiniMOD, MiniMODSummary, PPO2, PPO2Summary, ParseEANxError,
};
pub use crate::error::Error as GasError;
