//! Depth↔pressure conversion for variable dive environments.
//!
//! Standard dive tables are built on two fixed constants: $\pu{1.01325 bar}$ at the
//! surface and roughly $\pu{10.0 m}$ of seawater per bar of gauge pressure. That is
//! fine for a generic ocean dive, but the moment you leave those conditions —
//! high altitude, fresh water, the hypersaline Red Sea — both numbers shift,
//! and any deco calculation that ignores the shift is wrong.
//!
//! This module models those two variable parameters in [`DiveEnvironment`] and
//! provides the formulas and presets to compute them correctly.
//!
//! # The core equation
//!
//! Every depth↔pressure conversion in the library comes down to:
//!
//! $$
//! P_\text{abs} = P_\text{surface} + \frac{\text{depth}}{\text{water density}}
//! $$
//!
//! $P_\text{surface}$ is the atmospheric pressure at the dive site in bars.
//! $\text{water density}$ is the column height that produces one bar of gauge
//! pressure, expressed as metres per bar. [`DiveEnvironment`] holds exactly
//! these two values — nothing more.
//!
//! # Water density
//!
//! The mass of a one-metre water column determines how quickly pressure builds
//! with depth. ISO standard seawater ($\pu{35 ‰}$ salinity, $\pu{15 ^{\circ}C}$) has a density of
//! $\pu{1025 kg/m^3}$, giving roughly $\pu{9.95 m/bar}$. Real conditions span $\pu{8 ‰}$ in the
//! Baltic to $\pu{41 ‰}$ in the Red Sea, and temperatures from $\pu{2 ^{\circ}C}$ polar water to
//! $\pu{28 ^{\circ}C}$ tropical reefs. Both shift density enough to affect deco margins:
//! salt makes water heavier, heat makes it lighter.
//!
//! Rather than the full TEOS-10 equation of state, this module uses a linear
//! approximation anchored at the ISO reference point:
//!
//! $$
//! \rho(S, T) \approx 1000 + 0.8 \times S - 0.2 \times T \quad [\pu{kg/m^3}]
//! $$
//!
//! At ($\pu{35 ‰}$, $\pu{15 ^\circ C}$) this yields exactly $\pu{1025 kg/m^3}$. Across all practical
//! dive conditions ($S \in [\pu{0 ‰}, \pu{45 ‰}]$, $T \in [\pu{0 ^\circ C}, \pu{35 ^\circ C}]$) the error stays within
//! $\pm\pu{2 kg/m^3}$ — smaller than the uncertainty budget of any real deco model.
//!
//! # Surface pressure and altitude
//!
//! Atmosphere thins with altitude. The ICAO International Standard Atmosphere
//! gives the pressure at height h as:
//!
//! $$
//! P(h) = 101325 \times (1 - 2.25577 \times 10^{-5} \cdot h)^{5.25588} \quad [\text{Pa}]
//! $$
//!
//! At sea level this evaluates to $\pu{101325 Pa}$ ($\pu{1.01325 bar}$). At Lake Titicaca
//! ($\pu{3812 m}$) it drops to roughly $\pu{0.63 bar}$ — a surface pressure that pushes a
//! diver well into altitude-adjustment territory before they hit the water.
//!
//! The two coefficients come from atmospheric physics: $2.25577 \times 10^{-5}$ is
//! L/T₀, the temperature lapse rate ($\pu{0.0065 K/m}$) divided by ISA sea-level
//! temperature ($\pu{288.15 K}$); $5.25588$ is g·M/(R·L), standard gravity times
//! molar mass of dry air divided by the universal gas constant times the lapse
//! rate.
//!
//! # Presets
//!
//! | Constructor | Salinity | Temperature | Altitude |
//! |---|---|---|---|
//! | [`DiveEnvironment::standard`] | $\pu{35 ‰}$ ISO seawater | $\pu{15 ^\circ C}$ | sea level |
//! | [`DiveEnvironment::freshwater`] | $\pu{0 ‰}$ | $\pu{20 ^\circ C}$ | sea level |
//! | [`DiveEnvironment::ocean`] | from [`Ocean`] variant | from [`Ocean`] variant | sea level |
//! | [`DiveEnvironment::lake`] | $\pu{0 ‰}$ | from [`Lake`] variant | from [`Lake`] variant |
//!
//! Seventeen [`Ocean`] variants cover the five major oceans and twelve popular
//! dive seas, each keyed to representative surface salinity and temperature.
//! Fifteen [`Lake`] variants span sea-level systems (Florida springs, Yucatán
//! cenotes) through extreme-altitude sites (Licancabur at $\pu{5916 m}$, Ojos del
//! Salado at $\pu{6390 m}$).
//!
//! For anything not covered by a preset, the fallible constructors
//! [`DiveEnvironment::new`], [`DiveEnvironment::at_altitude`], and
//! [`DiveEnvironment::with_salinity_at_temperature`] accept validated explicit
//! values. The builder methods [`DiveEnvironment::with_altitude`],
//! [`DiveEnvironment::with_surface_pressure`], and
//! [`DiveEnvironment::with_water_density`] let you adjust a preset in place.
//!
//! ```
//! use dps::environment::{DiveEnvironment, Ocean, Lake};
//! use dps::units::{Bar, Meters, MetersPerBar};
//!
//! // Named ocean preset at sea level
//! let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
//!
//! // High-altitude freshwater lake
//! let titicaca = DiveEnvironment::lake(Lake::Titicaca);
//!
//! // Red Sea salinity at 500 m altitude — adjust a preset with a builder
//! let elevated = DiveEnvironment::ocean(Ocean::RedSea)
//!     .with_altitude(Meters::new(500.0))
//!     .unwrap();
//!
//! // Fully custom via validated constructor
//! let custom = DiveEnvironment::new(Bar::new(0.95), MetersPerBar::new(10.1)).unwrap();
//! ```

mod dive_environment;
mod lake;
mod ocean;
mod physics;

pub use self::dive_environment::{
    DiveEnvironment, DiveEnvironmentError, ParseDiveEnvironmentError,
};
pub use self::lake::Lake;
pub use self::ocean::Ocean;
