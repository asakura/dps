use std::fmt;

use crate::units::{Bar, CnsRatePerMinute, GramsPerLitre, Meters, OTUPerMinute, Percent};

use crate::environment::DiveEnvironment;

use super::blend::{BlendMethod, PartialPressure};
use super::components::GasComponents;
use super::constants::{EAN_MIN_O2, GAS_CONSTANT, STANDARD_TEMP_K};

mod detail;
pub use detail::EANxDetail;

mod equivalent_air_depth;
pub use equivalent_air_depth::{EAD, EADSummary};

mod equivalent_narcotic_depth;
pub use equivalent_narcotic_depth::{END, ENDSummary};

mod error;
pub use error::InvalidEANxError;

mod maximum_narcotic_depth;
pub use maximum_narcotic_depth::{MND, MNDSummary};

mod minimum_operating_depth;
pub use minimum_operating_depth::{MiniMOD, MiniMODSummary};

mod operating_depth;
pub use operating_depth::{MOD, MODSummary};

mod ppo2;
pub use ppo2::{PPO2, Ppo2Summary};

/// Enriched Air Nitrox, modelled by O₂ fraction and blending method.
///
/// The blend method determines the full gas composition (N₂, Ar, CO₂, traces)
/// from the O₂ fraction. See the [module documentation](crate::gas) for a comparison
/// of the three available methods.
///
/// Use the [`EANx`] type alias for the common partial-pressure case.
///
/// ```no_run
/// use dps::gas::{EANxBlend, Psa};
/// use dps::units::{Bar, Meters, Percent};
///
/// let psa32 = EANxBlend::new(Percent::new(0.32).unwrap(), Psa).unwrap();
///
/// let end = psa32.end_at(Meters::new(30.0));
/// let density = psa32.gas_density_at(Meters::new(30.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EANxBlend<M: BlendMethod> {
    fo2: Percent,
    method: M,
    env: DiveEnvironment,
}

/// Type alias for the most common case: partial-pressure blended nitrox.
///
/// Named mixes display their standard label; other fractions display as a
/// percentage. The label is determined by rounding the O₂ fraction to the
/// nearest whole percent.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::Percent;
/// let try_ean = |f| EANx::try_from(Percent::new(f).unwrap()).unwrap();
/// assert_eq!(try_ean(0.21).to_string(), "Air");
/// assert_eq!(try_ean(0.32).to_string(), "EANx 32");
/// assert_eq!(try_ean(1.00).to_string(), "Pure O₂");
/// assert_eq!(try_ean(0.25).to_string(), "25 %");
/// ```
pub type EANx = EANxBlend<PartialPressure>;

impl<M: BlendMethod> EANxBlend<M> {
    /// Constructs an [`EANxBlend`] from an O₂ fraction and a blending method.
    ///
    /// # Errors
    ///
    /// - [`InvalidEANxError::O2TooLow`] if `fo2 < 10 %`.
    /// - [`InvalidEANxError::BlendCeilingExceeded`] if `fo2` is above the
    ///   physical ceiling for `method` (PSA: ≈ 95.7 %).
    ///
    /// ```no_run
    /// use dps::gas::{EANxBlend, Psa, InvalidEANxError};
    /// use dps::units::Percent;
    ///
    /// assert!(EANxBlend::new(Percent::new(0.32).unwrap(), Psa).is_ok());
    /// assert!(matches!(
    ///     EANxBlend::new(Percent::new(0.09).unwrap(), Psa),
    ///     Err(InvalidEANxError::O2TooLow(_))
    /// ));
    /// assert!(matches!(
    ///     EANxBlend::new(Percent::new(0.99).unwrap(), Psa),
    ///     Err(InvalidEANxError::BlendCeilingExceeded(_))
    /// ));
    /// ```
    pub fn new(fo2: Percent, method: M) -> Result<Self, InvalidEANxError> {
        if fo2 < EAN_MIN_O2 {
            return Err(InvalidEANxError::O2TooLow(fo2));
        }

        if !method.is_valid_fo2(fo2.into()) {
            return Err(InvalidEANxError::BlendCeilingExceeded(fo2));
        }

        Ok(Self {
            fo2,
            method,
            env: DiveEnvironment::standard(),
        })
    }

    /// Returns a copy of this blend configured for the given dive environment.
    ///
    /// All subsequent depth calculations (`ppo2_at`, `mod_at`, `ead_at`, …) will
    /// use the provided surface pressure and water density instead of the
    /// ISO sea-level defaults.
    ///
    /// ```
    /// use dps::gas::EANx;
    /// use dps::environment::{DiveEnvironment, Lake};
    /// use dps::units::{Bar, Percent};
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    ///
    /// // At altitude, lower surface pressure means less absolute pressure at any given
    /// // depth, so ppO₂ stays within the limit to a greater physical depth — MOD is deeper.
    /// let titicaca = DiveEnvironment::lake(Lake::Titicaca);
    /// assert!(ean32.with_environment(titicaca).mod_at(Bar::new(1.4)).depth()
    ///     > ean32.mod_at(Bar::new(1.4)).depth());
    /// ```
    #[must_use]
    pub const fn with_environment(self, env: DiveEnvironment) -> Self {
        Self { env, ..self }
    }

    /// The dive environment used by this blend's depth calculations.
    ///
    /// Defaults to [`DiveEnvironment::standard`] (ISO sea-level seawater).
    ///
    /// ```
    /// use dps::gas::EANx;
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Percent;
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.env(), DiveEnvironment::standard());
    /// ```
    #[must_use]
    pub const fn env(self) -> DiveEnvironment {
        self.env
    }

    /// O₂ fraction.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(f64::from(ean32.fo2()), 0.32);
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// Full gas composition derived from the blend method.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// # use approx::assert_relative_eq;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// ```
    #[must_use]
    pub fn components(self) -> GasComponents {
        self.method.components(self.fo2.into())
    }

    /// N₂ fraction.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// // 68 % diluent, split as air — N₂ ≈ 67.3 %
    /// assert!(ean32.fn2() > 0.0 && ean32.fn2() < 0.68);
    /// ```
    #[must_use]
    pub fn fn2(self) -> f64 {
        self.components().n2()
    }

    /// Ar fraction.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// // Ar is present at air-trace levels in partial-pressure nitrox
    /// assert!(ean32.far() > 0.0 && ean32.far() < 0.01);
    /// ```
    #[must_use]
    pub fn far(self) -> f64 {
        self.components().ar()
    }

    /// CO₂ fraction.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.fco2() > 0.0 && ean32.fco2() < 0.001);
    /// ```
    #[must_use]
    pub fn fco2(self) -> f64 {
        self.components().co2()
    }

    /// Trace-gas fraction (Ne, He, Kr, …).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.fother() >= 0.0 && ean32.fother() < ean32.fco2());
    /// ```
    #[must_use]
    pub fn fother(self) -> f64 {
        self.components().other()
    }

    /// ppO₂ at the given depth.
    ///
    /// Formula: `ppO₂ = (depth / 9.948 m/bar + 1.013 bar) × FO₂`
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let air = EANx::try_from(Percent::new(0.21).unwrap()).unwrap();
    /// // Air at 30 m: (30/10 + 1) × 0.21 = 0.84 bar
    /// assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).pressure(), Bar::new(0.84), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn ppo2_at(self, depth: Meters) -> PPO2 {
        PPO2::new(self.fo2, depth, self.env)
    }

    /// Maximum Operating Depth for a given ppO₂ limit (O₂ toxicity constraint).
    ///
    /// Formula: `MOD = (ppO₂_max / FO₂ − 1.013 bar) × 9.948 m/bar`
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).depth(), Meters::new(33.44), epsilon = 0.01);
    /// ```
    ///
    /// # Panics
    ///
    /// Never: `EANxBlend` guarantees `fo2 >= 10 %` at construction.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "EANxBlend invariant: fo2 >= 10 % is checked at construction, so MOD::new never returns Err here"
    )]
    pub fn mod_at(self, ppo2_max: Bar) -> MOD {
        MOD::new(self.fo2, ppo2_max, self.env)
            .expect("fo2 guaranteed >= 10 % at EANxBlend construction")
    }

    /// Minimum Operating Depth for a given ppO₂ floor (hypoxia threshold).
    ///
    /// Returns 0 m for mixes that are normoxic or hyperoxic at the surface.
    /// Formula: `depth = (ppO₂_min / FO₂ − 1.013 bar) × 9.948 m/bar`, clamped to 0.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Hypoxic 10 % mix: minimum depth at ppO₂ 0.16 bar ≈ 5.84 m
    /// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
    /// assert_relative_eq!(h10.minimod_at(Bar::new(0.16)).depth(), Meters::new(5.84), epsilon = 0.01);
    ///
    /// // Normoxic air: no minimum depth
    /// let air = EANx::try_from(Percent::new(0.21).unwrap()).unwrap();
    /// assert_relative_eq!(air.minimod_at(Bar::new(0.16)).depth(), Meters::new(0.0), epsilon = 1e-9);
    /// ```
    ///
    /// # Panics
    ///
    /// Never: `EANxBlend` guarantees `fo2 >= 10 %` at construction.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "EANxBlend invariant: fo2 >= 10 % is checked at construction, so MiniMOD::new never returns Err here"
    )]
    pub fn minimod_at(self, ppo2_min: Bar) -> MiniMOD {
        MiniMOD::new(self.fo2, ppo2_min, self.env)
            .expect("fo2 guaranteed >= 10 % at EANxBlend construction")
    }

    /// Equivalent Narcotic Depth at a given actual depth.
    ///
    /// The depth on air that produces the same narcotic load (NOAA model:
    /// N₂ + 1.5 × Ar are narcotic; O₂ and CO₂ are excluded).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Air at any depth has END == actual depth
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.end_at(Meters::new(30.0)).end(), Meters::new(30.0), epsilon = 1e-6);
    ///
    /// // EANx 32 at 30 m has a shallower END than 30 m
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.end_at(Meters::new(30.0)).end() < Meters::new(30.0));
    /// ```
    #[must_use]
    pub fn end_at(self, depth: Meters) -> END {
        END::new(self.fo2, self.components().narcotic(), depth, self.env)
    }

    /// Maximum Narcotic Depth for a given END limit.
    ///
    /// The deepest depth at which the narcotic load does not exceed the
    /// equivalent narcotic effect of air at `end_limit`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let end_limit = Meters::new(30.0);
    /// let mnd = ean32.mnd_at(end_limit);
    /// // end_at(mnd) recovers the limit
    /// assert_relative_eq!(ean32.end_at(Meters::from(mnd)).end(), end_limit, epsilon = 1e-6);
    /// ```
    #[must_use]
    pub fn mnd_at(self, end_limit: Meters) -> MND {
        MND::new(self.fo2, self.components().narcotic(), end_limit, self.env)
    }

    /// Equivalent Air Depth at a given actual depth.
    ///
    /// The depth on air that produces the same N₂ partial pressure. Used to
    /// look up no-decompression limits from air tables.
    ///
    /// Formula: `EAD = ((depth + 10) × FN₂ / FN₂_air) − 10`
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Air's EAD equals actual depth
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.ead_at(Meters::new(30.0)).ead(), Meters::new(30.0), epsilon = 1e-6);
    ///
    /// // Enriched air has shallower EAD (less N₂ → less decompression obligation)
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.ead_at(Meters::new(30.0)).ead() < Meters::new(30.0));
    /// ```
    #[must_use]
    pub fn ead_at(self, depth: Meters) -> EAD {
        EAD::new(self.fo2, self.fn2(), depth, self.env)
    }

    /// Gas density in g/L at the given depth, at 20 °C (standard reference).
    ///
    /// Computed via the ideal gas law: `ρ = P × M / (R × T)`.
    /// Dense gas increases work of breathing and CO₂ retention risk at depth.
    ///
    /// Uses 1 atm (1.013 bar) as surface pressure and standard seawater density
    /// (1025 kg/m³), giving ≈ 1.204 g/L for dry air at 20 °C.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// // Density doubles at the depth where absolute pressure = 2 × P_surface (≈ 10.1 m)
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let d0 = ean32.gas_density_at(Meters::new(0.0));
    /// let d_double = ean32.gas_density_at(Meters::new(10.08));
    /// assert!((d_double / d0 - 2.0).abs() < 0.001);
    /// ```
    #[must_use]
    pub fn gas_density_at(self, depth: Meters) -> GramsPerLitre {
        let abs_pa =
            f64::from(depth / self.env.water_density() + self.env.surface_pressure()) * 1e5;

        GramsPerLitre::new(
            abs_pa * self.components().molar_mass() / (GAS_CONSTANT * STANDARD_TEMP_K),
        )
    }

    /// CNS O₂ toxicity rate in fraction of single-dive limit per minute.
    ///
    /// Uses the NOAA single-dive CNS exposure limit table. Multiply by exposure
    /// time in minutes to get the fraction of the CNS limit consumed.
    ///
    /// - Returns `0.0 CNS%/min` for `ppO₂ ≤ 0.5 bar` (below the CNS threshold).
    /// - Returns [`f64::INFINITY`] CNS%/min for `ppO₂ > 1.6 bar` (not recommended).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{CnsRatePerMinute, Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// // At MOD (33.75 m), ppO₂ = 1.4 bar → limit 150 min → rate ≈ 0.667 CNS%/min
    /// let rate = ean32.cns_rate_at(Meters::new(33.75));
    /// assert!((f64::from(rate) - 100.0 / 150.0).abs() < 1e-9);
    /// ```
    #[must_use]
    pub fn cns_rate_at(self, depth: Meters) -> CnsRatePerMinute {
        let limit = super::constants::cns_limit_minutes(self.ppo2_at(depth).pressure().into());

        if limit == 0.0 {
            CnsRatePerMinute::new(f64::INFINITY)
        } else {
            CnsRatePerMinute::new(100.0 / limit)
        }
    }

    /// OTU (Oxygen Tolerance Unit) accumulation rate per minute at the given depth.
    ///
    /// Formula: `OTU/min = (ppO₂ − 0.5)^0.83` when `ppO₂ > 0.5 bar`, else `0`.
    ///
    /// Multiply by exposure time in minutes; daily limit is ≈ 850 OTU.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, OTUPerMinute, Percent};
    /// // Below 0.5 bar threshold: zero OTU
    /// let air = EANx::try_from(Percent::new(0.21).unwrap()).unwrap();
    /// assert_eq!(air.otu_rate_at(Meters::new(0.0)), OTUPerMinute::new(0.0));
    ///
    /// // EANx 32 at 40 m: ppO₂ = 1.6 bar → (1.6 − 0.5)^0.83 ≈ 0.918 OTU/min
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.otu_rate_at(Meters::new(40.0)) > OTUPerMinute::new(0.0));
    /// ```
    #[must_use]
    pub fn otu_rate_at(self, depth: Meters) -> OTUPerMinute {
        let ppo2 = f64::from(self.ppo2_at(depth).pressure());

        if ppo2 <= 0.5 {
            OTUPerMinute::new(0.0)
        } else {
            OTUPerMinute::new((ppo2 - 0.5_f64).powf(0.83))
        }
    }

    /// Short human-readable name for the blend method used to produce this mix.
    ///
    /// ```no_run
    /// use dps::gas::{EANx, EANxBlend, Psa};
    /// use dps::units::Percent;
    ///
    /// let pp = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(pp.blend_name(), "partial pressure");
    ///
    /// let psa = EANxBlend::new(Percent::new(0.32).unwrap(), Psa).unwrap();
    /// assert_eq!(psa.blend_name(), "PSA");
    /// ```
    #[must_use]
    pub fn blend_name(self) -> &'static str {
        self.method.blend_name()
    }

    /// Returns a display wrapper that prints extended gas information.
    ///
    /// The wrapper's [`Display`](std::fmt::Display) shows the gas name, blend
    /// method, and the full component breakdown (O₂, N₂, Ar, CO₂, other).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// println!("{}", ean32.detail());
    /// ```
    #[must_use]
    pub fn detail(self) -> EANxDetail<M> {
        EANxDetail::from(self)
    }
}

impl EANxBlend<PartialPressure> {
    /// The optimal O₂ fraction for a target depth and ppO₂ limit.
    ///
    /// Returns the highest FO₂ (as a partial-pressure nitrox mix) that keeps
    /// ppO₂ at or below `ppo2_max` at `target_depth`, clamped to 100 % O₂.
    ///
    /// Returns `None` if the required FO₂ would be below the 10 % minimum
    /// (the target depth is beyond any breathable mix for that ppO₂ limit).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::{Bar, Meters};
    /// # use approx::assert_relative_eq;
    /// let best = EANx::best_mix(Meters::new(30.0), Bar::new(1.4), DiveEnvironment::standard()).unwrap();
    /// // FO₂ = 1.4 / (30/9.948 + 1.013) ≈ 0.347
    /// assert_relative_eq!(f64::from(best.fo2()), 0.347, epsilon = 0.001);
    ///
    /// // Verify that ppO₂ at target depth equals the limit
    /// assert_relative_eq!(best.ppo2_at(Meters::new(30.0)).pressure(), Bar::new(1.4), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn best_mix(target_depth: Meters, ppo2_max: Bar, env: DiveEnvironment) -> Option<Self> {
        let fo2 =
            (ppo2_max / (target_depth / env.water_density() + env.surface_pressure())).min(1.0);
        let pct = Percent::new(fo2)?;

        Self::new(pct, PartialPressure)
            .ok()
            .map(|blend| blend.with_environment(env))
    }
}

impl<M: BlendMethod> fmt::Display for EANxBlend<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", gas_name(self.fo2))
    }
}

pub(super) fn gas_name(fo2: Percent) -> impl fmt::Display {
    struct GasName(Percent);

    impl fmt::Display for GasName {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let pct = f64::from(self.0).mul_add(100.0, -0.5).ceil() as u8;
            let name = match pct {
                10 => "Hypoxic 10",
                12 => "Hypoxic 12",
                14 => "Hypoxic 14",
                16 => "Hypoxic 16",
                18 => "Hypoxic 18",
                21 => "Air",
                28 => "EANx 28",
                30 => "EANx 30",
                32 => "EANx 32",
                36 => "EANx 36",
                40 => "EANx 40",
                50 => "O₂ 50%",
                80 => "O₂ 80%",
                100 => "Pure O₂",
                _ => return write!(f, "{}", self.0),
            };

            write!(f, "{name}")
        }
    }

    GasName(fo2)
}

/// Constructs an [`EANx`] (partial-pressure nitrox) from an oxygen fraction.
///
/// # Errors
///
/// Returns [`InvalidEANxError::O2TooLow`] if `pct` is below 10 %.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::Percent;
/// assert!(EANx::try_from(Percent::new(0.32).unwrap()).is_ok());
/// assert!(EANx::try_from(Percent::new(0.10).unwrap()).is_ok());
/// assert!(EANx::try_from(Percent::new(0.09).unwrap()).is_err());
/// ```
impl TryFrom<Percent> for EANx {
    type Error = InvalidEANxError;

    fn try_from(pct: Percent) -> Result<Self, Self::Error> {
        Self::new(pct, PartialPressure)
    }
}

#[cfg(test)]
impl<M: BlendMethod + PartialEq> approx::AbsDiffEq for EANxBlend<M> {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.fo2.abs_diff_eq(&other.fo2, epsilon)
    }
}

#[cfg(test)]
impl<M: BlendMethod + PartialEq> approx::RelativeEq for EANxBlend<M> {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.fo2.relative_eq(&other.fo2, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::DiveEnvironment;
    use crate::gas::blend::Psa;
    use crate::gas::constants::AIR_O2;
    use crate::units::{Bar, CnsRatePerMinute, GramsPerLitre, Meters, OTUPerMinute, Percent};
    use approx::assert_relative_eq;
    use color_eyre::{Result, eyre::eyre};
    use rstest::*;

    fn ean(fraction: f64) -> Result<EANx> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

        Ok(EANx::try_from(pct)?)
    }

    fn ean_psa(fraction: f64) -> Result<EANxBlend<Psa>> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

        Ok(EANxBlend::new(pct, Psa)?)
    }

    fn bad_ean(fo2: f64) -> EANx {
        EANxBlend {
            fo2: Percent::new(fo2).unwrap_or_else(|| unreachable!("fo2 is in [0.0, 1.0]")),
            method: PartialPressure,
            env: DiveEnvironment::standard(),
        }
    }

    mod fo2 {
        use super::*;

        #[rstest]
        fn fo2_matches_fraction() -> Result<()> {
            assert_relative_eq!(
                ean(0.21)?.fo2(),
                Percent::new(0.21).ok_or_else(|| eyre!("invalid"))?
            );
            assert_relative_eq!(
                ean(0.32)?.fo2(),
                Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?
            );
            assert_relative_eq!(
                ean(1.0)?.fo2(),
                Percent::new(1.0).ok_or_else(|| eyre!("invalid"))?
            );

            Ok(())
        }
    }

    mod mod_at {
        use super::*;

        #[test]
        fn mod_at_eanx32_1_4_bar() -> Result<()> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?;
            let expected = (Bar::new(1.4) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_eanx40_1_4_bar() -> Result<()> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.40).ok_or_else(|| eyre!("invalid"))?;
            let expected = (Bar::new(1.4) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(0.40)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_pure_o2_1_6_bar() -> Result<()> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(1.0).ok_or_else(|| eyre!("invalid"))?;
            let expected = (Bar::new(1.6) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(1.0)?.mod_at(Bar::new(1.6)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_clamps_to_zero_when_negative() -> Result<()> {
            assert_relative_eq!(
                ean(1.0)?.mod_at(Bar::new(0.5)).depth(),
                Meters::new(0.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<()> {
            let fo2 = Percent::new(0.32).ok_or_else(|| eyre!("0.32 is in [0.0, 1.0]"))?;
            assert_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).fo2(), fo2);

            Ok(())
        }

        #[test]
        fn ppo2_max_is_preserved() -> Result<()> {
            let ppo2 = Bar::new(1.6);
            assert_eq!(ean(0.32)?.mod_at(ppo2).ppo2_max(), ppo2);

            Ok(())
        }

        #[test]
        #[should_panic(expected = "fo2 guaranteed >= 10 %")]
        fn panics_if_fo2_invariant_violated() {
            let _ = bad_ean(0.05).mod_at(Bar::new(1.4));
        }
    }

    mod minimod_at {
        use super::*;

        #[test]
        fn normoxic_mix_has_zero_minimod() -> Result<()> {
            assert_relative_eq!(
                ean(0.21)?.minimod_at(Bar::new(0.16)).depth(),
                Meters::new(0.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn hypoxic_10_percent_at_0_16_bar() -> Result<()> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.10).ok_or_else(|| eyre!("invalid"))?;
            let expected = (Bar::new(0.16) / fo2 - env.surface_pressure()).max(Bar::new(0.0))
                * env.water_density();

            assert_relative_eq!(
                ean(0.10)?.minimod_at(Bar::new(0.16)).depth(),
                expected,
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<()> {
            let fo2 = Percent::new(0.10).ok_or_else(|| eyre!("0.10 is in [0.0, 1.0]"))?;
            assert_eq!(ean(0.10)?.minimod_at(Bar::new(0.16)).fo2(), fo2);

            Ok(())
        }

        #[test]
        fn ppo2_min_is_preserved() -> Result<()> {
            let ppo2 = Bar::new(0.16);
            assert_eq!(ean(0.10)?.minimod_at(ppo2).ppo2_min(), ppo2);

            Ok(())
        }

        #[test]
        fn into_meters_gives_depth() -> Result<()> {
            let m = ean(0.10)?.minimod_at(Bar::new(0.16));
            assert_eq!(Meters::from(m), m.depth());

            Ok(())
        }

        #[test]
        #[should_panic(expected = "fo2 guaranteed >= 10 %")]
        fn panics_if_fo2_invariant_violated() {
            let _ = bad_ean(0.05).minimod_at(Bar::new(0.16));
        }
    }

    mod gas_density_at {
        use super::*;

        #[test]
        fn air_at_surface_is_approximately_1_20_g_per_l() -> Result<()> {
            let density = ean(f64::from(AIR_O2))?.gas_density_at(Meters::new(0.0));
            assert_relative_eq!(density, GramsPerLitre::new(1.204), epsilon = 0.002);

            Ok(())
        }

        #[test]
        fn density_doubles_at_one_atmosphere_depth() -> Result<()> {
            let env = DiveEnvironment::standard();
            let surface = ean(0.32)?.gas_density_at(Meters::new(0.0));
            // pressure doubles at depth = surface_pressure × water_density ≈ 10.08 m
            let double_depth = env.surface_pressure() * env.water_density();
            let at_double = ean(0.32)?.gas_density_at(double_depth);

            assert_relative_eq!(at_double, surface * 2.0, epsilon = 1e-9);

            Ok(())
        }
    }

    mod cns_rate_at {
        use super::*;

        #[test]
        fn zero_below_threshold() -> Result<()> {
            assert_relative_eq!(
                ean(0.21)?.cns_rate_at(Meters::new(0.0)),
                CnsRatePerMinute::new(0.0)
            );

            Ok(())
        }

        #[test]
        fn at_1_4_bar_limit_is_150_minutes() -> Result<()> {
            // EANx32 at 32.5 m: ppO₂ = (32.5/9.948 + 1.013) × 0.32 ≈ 1.370 bar
            // Falls in the 1.30–1.40 range → 150 min limit.
            assert_relative_eq!(
                ean(0.32)?.cns_rate_at(Meters::new(32.5)),
                CnsRatePerMinute::new(100.0 / 150.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn above_1_6_bar_is_infinite() -> Result<()> {
            // Pure O₂ at 7 m: ppO₂ = (7/9.948 + 1.013) × 1.0 ≈ 1.717 bar
            assert_eq!(
                ean(1.0)?.cns_rate_at(Meters::new(7.0)),
                CnsRatePerMinute::new(f64::INFINITY)
            );

            Ok(())
        }
    }

    mod otu_rate_at {
        use super::*;

        #[test]
        fn zero_below_0_5_bar() -> Result<()> {
            // Air at surface: ppO₂ ≈ 0.21 bar < 0.5 → zero OTU
            assert_relative_eq!(
                ean(0.21)?.otu_rate_at(Meters::new(0.0)),
                OTUPerMinute::new(0.0)
            );

            Ok(())
        }

        #[test]
        fn follows_noaa_formula() -> Result<()> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(40.0);
            let fo2 = Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?;
            let ppo2 = f64::from((depth / env.water_density() + env.surface_pressure()) * fo2);
            let expected = OTUPerMinute::new((ppo2 - 0.5_f64).powf(0.83));

            assert_relative_eq!(ean(0.32)?.otu_rate_at(depth), expected, epsilon = 1e-9);

            Ok(())
        }
    }

    mod best_mix {
        use super::*;

        #[test]
        fn at_30m_1_4_bar() -> Result<()> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(30.0);
            let ppo2_max = Bar::new(1.4);
            let expected_fo2 = ppo2_max / (depth / env.water_density() + env.surface_pressure());
            let best = EANx::best_mix(depth, ppo2_max, env)
                .ok_or_else(|| eyre!("fo2 is above the 10 % minimum"))?;

            assert_relative_eq!(f64::from(best.fo2()), expected_fo2.min(1.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn ppo2_at_target_equals_limit() -> Result<()> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(40.0);
            let ppo2_max = Bar::new(1.4);
            let best = EANx::best_mix(depth, ppo2_max, env)
                .ok_or_else(|| eyre!("fo2 = 0.28 is above the 10 % minimum"))?;

            assert_relative_eq!(best.ppo2_at(depth).pressure(), ppo2_max, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn shallow_depth_clamps_to_pure_o2() -> Result<()> {
            // fo2 = 1.4 / (3/9.948 + 1.013) ≈ 1.065 > 1.0 → clamps to 1.0
            let best = EANx::best_mix(Meters::new(3.0), Bar::new(1.4), DiveEnvironment::standard())
                .ok_or_else(|| eyre!("fo2 is above the 10 % minimum"))?;

            assert_relative_eq!(f64::from(best.fo2()), 1.0, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn very_deep_returns_none() {
            // At extreme depth, required fo2 would be below 10 % minimum
            assert!(
                EANx::best_mix(
                    Meters::new(200.0),
                    Bar::new(1.4),
                    DiveEnvironment::standard()
                )
                .is_none()
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_air() -> Result<()> {
            assert_eq!(ean(0.21)?.to_string(), "Air");
            assert_eq!(ean(0.215)?.to_string(), "Air");
            assert_ne!(ean(0.205)?.to_string(), "Air");

            Ok(())
        }

        #[test]
        fn display_named_nitrox_mixes() -> Result<()> {
            assert_eq!(ean(0.28)?.to_string(), "EANx 28");
            assert_eq!(ean(0.30)?.to_string(), "EANx 30");
            assert_eq!(ean(0.32)?.to_string(), "EANx 32");
            assert_eq!(ean(0.36)?.to_string(), "EANx 36");
            assert_eq!(ean(0.40)?.to_string(), "EANx 40");

            Ok(())
        }

        #[test]
        fn display_high_o2_mixes() -> Result<()> {
            assert_eq!(ean(0.50)?.to_string(), "O₂ 50%");
            assert_eq!(ean(0.80)?.to_string(), "O₂ 80%");

            Ok(())
        }

        #[test]
        fn display_hypoxic_mixes() -> Result<()> {
            assert_eq!(ean(0.10)?.to_string(), "Hypoxic 10");
            assert_eq!(ean(0.12)?.to_string(), "Hypoxic 12");
            assert_eq!(ean(0.14)?.to_string(), "Hypoxic 14");
            assert_eq!(ean(0.16)?.to_string(), "Hypoxic 16");
            assert_eq!(ean(0.18)?.to_string(), "Hypoxic 18");

            Ok(())
        }

        #[test]
        fn display_pure_o2() -> Result<()> {
            assert_eq!(ean(1.0)?.to_string(), "Pure O₂");

            Ok(())
        }

        #[test]
        fn display_unnamed_mix_shows_fraction() -> Result<()> {
            assert_eq!(ean(0.25)?.to_string(), "25 %");
            assert_eq!(ean(0.33)?.to_string(), "33 %");

            Ok(())
        }

        #[test]
        fn display_is_blend_method_agnostic() -> Result<()> {
            assert_eq!(ean(0.32)?.to_string(), ean_psa(0.32)?.to_string());

            Ok(())
        }
    }

    mod try_from_percent {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case(0.21)]
        #[case(0.32)]
        #[case(0.40)]
        #[case(1.0)]
        fn try_from_percent_preserves_fraction(#[case] fraction: f64) -> Result<()> {
            let pct = Percent::new(fraction)
                .ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

            assert_eq!(EANx::try_from(pct)?.fo2(), pct);

            Ok(())
        }

        #[test]
        fn try_from_percent_rejects_below_minimum() -> Result<()> {
            assert!(EANx::try_from(Percent::new(0.09).ok_or_else(|| eyre!("invalid"))?).is_err());

            Ok(())
        }

        #[test]
        fn try_from_percent_accepts_fraction_that_rounds_into_valid_range() -> Result<()> {
            assert!(EANx::try_from(Percent::new(0.316).ok_or_else(|| eyre!("invalid"))?).is_ok());

            Ok(())
        }
    }
}
