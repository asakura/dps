mod detail;
mod equivalent_air_depth;
mod equivalent_narcotic_depth;
mod error;
mod maximum_narcotic_depth;
mod minimum_operating_depth;
mod operating_depth;
mod ppo2;

pub use self::detail::EANxDetail;
pub use self::equivalent_air_depth::{EAD, EADSummary};
pub use self::equivalent_narcotic_depth::{END, ENDSummary};
pub use self::error::{InvalidEANxError, ParseEANxError};
pub use self::maximum_narcotic_depth::{MND, MNDSummary};
pub use self::minimum_operating_depth::{MiniMOD, MiniMODSummary};
pub use self::operating_depth::{MOD, MODSummary};
pub use self::ppo2::{PPO2, PPO2Summary};
use super::{
    blend::{BlendMethod, PartialPressure},
    components::GasComponents,
    constants::{EAN_MIN_O2, GAS_CONSTANT, STANDARD_TEMP_K},
};

use dps_environment::DiveEnvironment;
use dps_units::{Bar, CnsRatePerMinute, GramsPerLitre, Meters, OTUPerMinute, Percent};

use std::{fmt, str::FromStr};

/// Enriched Air Nitrox, modelled by $\ce{O2}$ fraction and blending method.
///
/// The blend method determines the full gas composition ($\ce{N2}$, $\ce{Ar}$, $\ce{CO2}$, traces)
/// from the $\ce{O2}$ fraction. See the [module documentation](crate) for a comparison
/// of the three available methods.
///
/// Use the [`EANx`] type alias for the common partial-pressure case.
///
/// ```
/// use dps_gas::prelude::{EANxBlend, Psa};
/// use dps_units::{Bar, Meters, Percent};
///
/// let psa32 = EANxBlend::new(Percent::new(0.32).unwrap(), Psa).unwrap();
///
/// let end = psa32.end_at(Meters::new(30.0));
/// assert!(end.depth() < Meters::new(30.0));
/// let density = psa32.gas_density_at(Meters::new(30.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        try_from = "EANxBlendShadow<M>",
        bound(deserialize = "M: ::serde::Deserialize<'de>")
    )
)]
pub struct EANxBlend<M: BlendMethod> {
    fo2: Percent,
    method: M,
    env: DiveEnvironment,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct EANxBlendShadow<M> {
    fo2: Percent,
    method: M,
    env: DiveEnvironment,
}

#[cfg(feature = "serde")]
impl<M: BlendMethod> TryFrom<EANxBlendShadow<M>> for EANxBlend<M> {
    type Error = String;

    fn try_from(shadow: EANxBlendShadow<M>) -> Result<Self, Self::Error> {
        Self::new(shadow.fo2, shadow.method)
            .map(|blend| blend.with_environment(shadow.env))
            .map_err(|e| e.to_string())
    }
}

/// Type alias for the most common case: partial-pressure blended nitrox.
///
/// Named mixes display their standard label; other fractions display as a
/// percentage. The label is determined by rounding the $\ce{O2}$ fraction to the
/// nearest whole percent.
///
/// ```
/// use dps_gas::prelude::EANx;
/// use dps_units::Percent;
/// let try_ean = |f| EANx::try_from(Percent::new(f).unwrap()).unwrap();
/// assert_eq!(try_ean(0.21).to_string(), "Air");
/// assert_eq!(try_ean(0.32).to_string(), "EANx 32");
/// assert_eq!(try_ean(1.00).to_string(), "Pure O₂");
/// assert_eq!(try_ean(0.25).to_string(), "25%");
/// ```
pub type EANx = EANxBlend<PartialPressure>;

impl<M: BlendMethod> EANxBlend<M> {
    /// Constructs an [`EANxBlend`] from an $\ce{O2}$ fraction and a blending method.
    ///
    /// # Errors
    ///
    /// - [`InvalidEANxError::O2TooLow`] if `fo2 < 10%`.
    /// - [`InvalidEANxError::BlendCeilingExceeded`] if `fo2` is above the
    ///   physical ceiling for `method` (PSA: $\approx 95.7\\%$).
    ///
    /// ```
    /// use dps_gas::prelude::{EANxBlend, Psa, InvalidEANxError};
    /// use dps_units::Percent;
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
    /// use dps_gas::prelude::EANx;
    /// use dps_environment::{DiveEnvironment, Lake};
    /// use dps_units::{Bar, Percent};
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
    /// use dps_gas::prelude::EANx;
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Percent;
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.env(), DiveEnvironment::standard());
    /// ```
    #[must_use]
    pub const fn env(self) -> DiveEnvironment {
        self.env
    }

    /// $\ce{O2}$ fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
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
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// # use approx::assert_relative_eq;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// ```
    #[must_use]
    pub fn components(self) -> GasComponents {
        self.method.components(self.fo2.into())
    }

    /// $\ce{N2}$ fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// // 68% diluent, split as air — N₂ ≈ 67.3%
    /// assert!(ean32.fn2() > 0.0 && ean32.fn2() < 0.68);
    /// ```
    #[must_use]
    pub fn fn2(self) -> f64 {
        self.components().n2()
    }

    /// $\ce{Ar}$ fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// // Ar is present at air-trace levels in partial-pressure nitrox
    /// assert!(ean32.far() > 0.0 && ean32.far() < 0.01);
    /// ```
    #[must_use]
    pub fn far(self) -> f64 {
        self.components().ar()
    }

    /// $\ce{CO2}$ fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.fco2() > 0.0 && ean32.fco2() < 0.001);
    /// ```
    #[must_use]
    pub fn fco2(self) -> f64 {
        self.components().co2()
    }

    /// Trace-gas fraction ($\ce{Ne}$, $\ce{He}$, $\ce{Kr}$, …).
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.fother() >= 0.0 && ean32.fother() < ean32.fco2());
    /// ```
    #[must_use]
    pub fn fother(self) -> f64 {
        self.components().other()
    }

    /// $\text{pp}\ce{O2}$ at the given depth.
    ///
    /// Formula: $\text{pp}\ce{O2} = \bigl(\text{depth} / \pu{9.948 m/bar} + \pu{1.013 bar}\bigr) \times \text{F}\ce{O2}$
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let air = EANx::try_from(Percent::new(0.21).unwrap()).unwrap();
    /// // Air at 30 m: (30/10 + 1) × 0.21 = 0.84 bar
    /// assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).pressure(), Bar::new(0.84), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn ppo2_at(self, depth: Meters) -> PPO2 {
        PPO2::new(self.fo2, depth, self.env)
    }

    /// Maximum Operating Depth for a given $\text{pp}\ce{O2}$ limit ($\ce{O2}$ toxicity constraint).
    ///
    /// Formula: $\text{MOD} = \bigl(\text{pp}\ce{O2}_{\text{max}} / \text{F}\ce{O2} - \pu{1.013 bar}\bigr) \times \pu{9.948 m/bar}$
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).depth(), Meters::new(33.44), epsilon = 0.01);
    /// ```
    ///
    /// # Panics
    ///
    /// Never: `EANxBlend` guarantees $\text{F}\ce{O2} \geq 10\\%$ at construction.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "EANxBlend invariant: fo2 >= 10% is checked at construction, so MOD::new never returns Err here"
    )]
    pub fn mod_at(self, ppo2_max: Bar) -> MOD {
        MOD::new(self.fo2, ppo2_max, self.env)
            .expect("fo2 guaranteed >= 10% at EANxBlend construction")
    }

    /// Minimum Operating Depth for a given $\text{pp}\ce{O2}$ floor (hypoxia threshold).
    ///
    /// Returns 0 m for mixes that are normoxic or hyperoxic at the surface.
    /// Formula: $\text{depth} = \bigl(\text{pp}\ce{O2}_{\text{min}} / \text{F}\ce{O2} - \pu{1.013 bar}\bigr) \times \pu{9.948 m/bar}$, clamped to 0.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Hypoxic 10% mix: minimum depth at ppO₂ 0.16 bar ≈ 5.84 m
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
    /// Never: `EANxBlend` guarantees $\text{F}\ce{O2} \geq 10\\%$ at construction.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "EANxBlend invariant: fo2 >= 10% is checked at construction, so MiniMOD::new never returns Err here"
    )]
    pub fn minimod_at(self, ppo2_min: Bar) -> MiniMOD {
        MiniMOD::new(self.fo2, ppo2_min, self.env)
            .expect("fo2 guaranteed >= 10% at EANxBlend construction")
    }

    /// Equivalent Narcotic Depth at a given actual depth.
    ///
    /// The depth on air that produces the same narcotic load (NOAA model:
    /// $\ce{N2}$ + 1.5 × Ar are narcotic; $\ce{O2}$ and $\ce{CO2}$ are excluded).
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Air at any depth has END == actual depth
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.end_at(Meters::new(30.0)).depth(), Meters::new(30.0), epsilon = 1e-6);
    ///
    /// // EANx 32 at 30 m has a shallower END than 30 m
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.end_at(Meters::new(30.0)).depth() < Meters::new(30.0));
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
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let end_limit = Meters::new(30.0);
    /// let mnd = ean32.mnd_at(end_limit);
    /// // end_at(mnd) recovers the limit
    /// assert_relative_eq!(ean32.end_at(Meters::from(mnd)).depth(), end_limit, epsilon = 1e-6);
    /// ```
    #[must_use]
    pub fn mnd_at(self, end_limit: Meters) -> MND {
        MND::new(self.fo2, self.components().narcotic(), end_limit, self.env)
    }

    /// Equivalent Air Depth at a given actual depth.
    ///
    /// The depth on air that produces the same $\ce{N2}$ partial pressure. Used to
    /// look up no-decompression limits from air tables.
    ///
    /// Formula: $\text{EAD} = \bigl((\text{depth} + 10) \times F_{\ce{N2}} / F_{\ce{N2},\text{air}}\bigr) - 10$
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// // Air's EAD equals actual depth
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.ead_at(Meters::new(30.0)).depth(), Meters::new(30.0), epsilon = 1e-6);
    ///
    /// // Enriched air has shallower EAD (less N₂ → less decompression obligation)
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(ean32.ead_at(Meters::new(30.0)).depth() < Meters::new(30.0));
    /// ```
    #[must_use]
    pub fn ead_at(self, depth: Meters) -> EAD {
        EAD::new(self.fo2, self.fn2(), depth, self.env)
    }

    /// Gas density in g/L at the given depth, at 20 °C (standard reference).
    ///
    /// Computed via the ideal gas law: `ρ = P × M / (R × T)`.
    /// Dense gas increases work of breathing and $\ce{CO2}$ retention risk at depth.
    ///
    /// Uses 1 atm (1.013 bar) as surface pressure and standard seawater density
    /// ($\pu{1025 kg/m^3}$), giving $\approx \pu{1.204 g/L}$ for dry air at $\pu{20 ^\circ C}$.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Meters, Percent};
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

    /// CNS $\ce{O2}$ toxicity rate in fraction of single-dive limit per minute.
    ///
    /// Uses the NOAA single-dive CNS exposure limit table. Multiply by exposure
    /// time in minutes to get the fraction of the CNS limit consumed.
    ///
    /// - Returns `0.0 CNS%/min` for $\text{pp}\ce{O2} \leq \pu{0.5 bar}$ (below the CNS threshold).
    /// - Returns [`f64::INFINITY`] CNS%/min for $\text{pp}\ce{O2} > \pu{1.6 bar}$ (not recommended).
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{CnsRatePerMinute, Meters, Percent};
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
    /// Formula: $(\text{pp}\ce{O2} - \pu{0.5 bar})^{0.83}$ when $\text{pp}\ce{O2} > \pu{0.5 bar}$, else $0$.
    ///
    /// Multiply by exposure time in minutes; daily limit is $\approx 850\,\text{OTU}$.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Meters, OTUPerMinute, Percent};
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
    /// ```
    /// use dps_gas::prelude::{EANx, EANxBlend, Psa};
    /// use dps_units::Percent;
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
    /// method, and the full component breakdown ($\ce{O2}$, $\ce{N2}$, $\ce{Ar}$, $\ce{CO2}$, other).
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
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
    /// The optimal $\ce{O2}$ fraction for a target depth and $\text{pp}\ce{O2}$ limit.
    ///
    /// Returns the highest $\text{F}\ce{O2}$ (as a partial-pressure nitrox mix) that keeps
    /// $\text{pp}\ce{O2}$ at or below `ppo2_max` at `target_depth`, clamped to 100% $\ce{O2}$.
    ///
    /// # Errors
    ///
    /// Returns `Err(InvalidEANxError::O2TooLow)` if the required $\text{F}\ce{O2}$ would be
    /// below $\text{F}\ce{O2} \geq 10\\%$ (the target depth is beyond any breathable mix
    /// for that $\text{pp}\ce{O2}$ limit).
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::{Bar, Meters};
    /// # use approx::assert_relative_eq;
    /// let best = EANx::best_mix(Meters::new(30.0), Bar::new(1.4), DiveEnvironment::standard()).unwrap();
    /// // FO₂ = 1.4 / (30/9.948 + 1.013) ≈ 0.347
    /// assert_relative_eq!(f64::from(best.fo2()), 0.347, epsilon = 0.001);
    ///
    /// // Verify that ppO₂ at target depth equals the limit
    /// assert_relative_eq!(best.ppo2_at(Meters::new(30.0)).pressure(), Bar::new(1.4), epsilon = 1e-9);
    /// ```
    pub fn best_mix(
        target_depth: Meters,
        ppo2_max: Bar,
        env: DiveEnvironment,
    ) -> Result<Self, InvalidEANxError> {
        let fo2 =
            (ppo2_max / (target_depth / env.water_density() + env.surface_pressure())).min(1.0);
        let pct = Percent::new(fo2)?;

        Self::new(pct, PartialPressure).map(|blend| blend.with_environment(env))
    }
}

impl<M: BlendMethod> fmt::Display for EANxBlend<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", gas_name(self.fo2))
    }
}

pub fn gas_name(fo2: Percent) -> impl fmt::Display {
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
/// Returns [`InvalidEANxError::O2TooLow`] if `pct` is below $\text{F}\ce{O2} \geq 10\\%$.
///
/// ```
/// use dps_gas::prelude::EANx;
/// use dps_units::Percent;
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

/// Returns air ($21\\%$ $\ce{O2}$, partial-pressure blended).
///
/// # Examples
///
/// ```
/// use dps_gas::prelude::EANx;
/// use dps_units::Percent;
///
/// let air = EANx::default();
/// assert_eq!(air.to_string(), "Air");
/// assert_eq!(air.fo2(), Percent::new(0.21).unwrap());
/// ```
impl Default for EANx {
    #[expect(
        clippy::expect_used,
        reason = "0.21 is within Percent bounds and satisfies EAN_MIN_O2; PartialPressure has no ceiling"
    )]
    fn default() -> Self {
        Self::new(
            Percent::new(0.21).expect("0.21 is a valid O₂ fraction"),
            PartialPressure,
        )
        .expect("air satisfies EAN_MIN_O2 and PartialPressure has no ceiling")
    }
}

/// Parses an [`EANx`] blend from its display representation.
///
/// Accepts the formats produced by [`Display`](std::fmt::Display):
/// `"Air"`, `"EANx N"`, `"Hypoxic N"`, `"O₂ N%"`, `"Pure O₂"`, and the
/// fallback `"N%"` / `"N.d%"` percentage format from
/// [`Percent`](dps_units::Percent)'s own display.
///
/// # Errors
///
/// Returns [`InvalidEANxError`] if the string does not match any known format or
/// the resulting $\ce{O2}$ fraction is outside the valid [`EANx`] range ($\text{F}\ce{O2} \geq 10\\%$).
///
/// # Examples
///
/// ```
/// use dps_gas::prelude::EANx;
///
/// assert_eq!("Air".parse::<EANx>().unwrap().to_string(),    "Air");
/// assert_eq!("EANx 32".parse::<EANx>().unwrap().to_string(), "EANx 32");
/// assert_eq!("Pure O₂".parse::<EANx>().unwrap().to_string(), "Pure O₂");
/// assert_eq!("25%".parse::<EANx>().unwrap().to_string(),    "25%");
/// assert!("invalid".parse::<EANx>().is_err());
/// ```
impl FromStr for EANx {
    type Err = InvalidEANxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fo2_frac: f64 = if s == "Air" {
            0.21
        } else if s == "Pure O₂" {
            1.0
        } else if let Some(n_str) = s.strip_prefix("EANx ") {
            let n: u8 = n_str.parse().map_err(|_| ParseEANxError)?;

            f64::from(n) / 100.0
        } else if let Some(n_str) = s.strip_prefix("Hypoxic ") {
            let n: u8 = n_str.parse().map_err(|_| ParseEANxError)?;

            f64::from(n) / 100.0
        } else if let Some(inner) = s.strip_prefix("O₂ ").and_then(|t| t.strip_suffix('%')) {
            let n: u8 = inner.parse().map_err(|_| ParseEANxError)?;

            f64::from(n) / 100.0
        } else {
            let pct: Percent = s.parse().map_err(|_| ParseEANxError)?;

            return Self::try_from(pct);
        };

        let pct = Percent::new(fo2_frac).map_err(|_| ParseEANxError)?;

        Self::try_from(pct)
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

    use crate::blend::Psa;
    use crate::constants::AIR_O2;

    use dps_environment::DiveEnvironment;
    use dps_units::{Bar, CnsRatePerMinute, GramsPerLitre, Meters, OTUPerMinute, Percent};

    use approx::assert_relative_eq;
    use rstest::*;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;

        EANx::try_from(pct)
    }

    fn ean_psa(fraction: f64) -> Result<EANxBlend<Psa>, InvalidEANxError> {
        let pct = Percent::new(fraction)?;

        EANxBlend::new(pct, Psa)
    }

    mod fo2 {
        use super::*;

        #[rstest]
        fn fo2_matches_fraction() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(ean(0.21)?.fo2(), Percent::new(0.21)?);
            assert_relative_eq!(ean(0.32)?.fo2(), Percent::new(0.32)?);
            assert_relative_eq!(ean(1.0)?.fo2(), Percent::new(1.0)?);

            Ok(())
        }
    }

    mod mod_at {
        use super::*;

        #[test]
        fn mod_at_eanx32_1_4_bar() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.32)?;
            let expected = (Bar::new(1.4) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_eanx40_1_4_bar() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.40)?;
            let expected = (Bar::new(1.4) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(0.40)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_pure_o2_1_6_bar() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(1.0)?;
            let expected = (Bar::new(1.6) / fo2 - env.surface_pressure()) * env.water_density();

            assert_relative_eq!(ean(1.0)?.mod_at(Bar::new(1.6)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_clamps_to_zero_when_negative() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(
                ean(1.0)?.mod_at(Bar::new(0.5)).depth(),
                Meters::new(0.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.32)?;
            assert_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).fo2(), fo2);

            Ok(())
        }

        #[test]
        fn ppo2_max_is_preserved() -> Result<(), InvalidEANxError> {
            let ppo2 = Bar::new(1.6);
            assert_eq!(ean(0.32)?.mod_at(ppo2).ppo2_max(), ppo2);

            Ok(())
        }
    }

    mod minimod_at {
        use super::*;

        #[test]
        fn normoxic_mix_has_zero_minimod() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(
                ean(0.21)?.minimod_at(Bar::new(0.16)).depth(),
                Meters::new(0.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn hypoxic_10_percent_at_0_16_bar() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.10)?;
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
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.10)?;
            assert_eq!(ean(0.10)?.minimod_at(Bar::new(0.16)).fo2(), fo2);

            Ok(())
        }

        #[test]
        fn ppo2_min_is_preserved() -> Result<(), InvalidEANxError> {
            let ppo2 = Bar::new(0.16);
            assert_eq!(ean(0.10)?.minimod_at(ppo2).ppo2_min(), ppo2);

            Ok(())
        }

        #[test]
        fn into_meters_gives_depth() -> Result<(), InvalidEANxError> {
            let m = ean(0.10)?.minimod_at(Bar::new(0.16));
            assert_eq!(Meters::from(m), m.depth());

            Ok(())
        }
    }

    mod gas_density_at {
        use super::*;

        #[test]
        fn air_at_surface_is_approximately_1_20_g_per_l() -> Result<(), InvalidEANxError> {
            let density = ean(f64::from(AIR_O2))?.gas_density_at(Meters::new(0.0));
            assert_relative_eq!(density, GramsPerLitre::new(1.204), epsilon = 0.002);

            Ok(())
        }

        #[test]
        fn density_doubles_at_one_atmosphere_depth() -> Result<(), InvalidEANxError> {
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
        fn zero_below_threshold() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(
                ean(0.21)?.cns_rate_at(Meters::new(0.0)),
                CnsRatePerMinute::new(0.0)
            );

            Ok(())
        }

        #[test]
        fn at_1_4_bar_limit_is_150_minutes() -> Result<(), InvalidEANxError> {
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
        fn above_1_6_bar_is_infinite() -> Result<(), InvalidEANxError> {
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
        fn zero_below_0_5_bar() -> Result<(), InvalidEANxError> {
            // Air at surface: ppO₂ ≈ 0.21 bar < 0.5 → zero OTU
            assert_relative_eq!(
                ean(0.21)?.otu_rate_at(Meters::new(0.0)),
                OTUPerMinute::new(0.0)
            );

            Ok(())
        }

        #[test]
        fn follows_noaa_formula() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(40.0);
            let fo2 = Percent::new(0.32)?;
            let ppo2 = f64::from((depth / env.water_density() + env.surface_pressure()) * fo2);
            let expected = OTUPerMinute::new((ppo2 - 0.5_f64).powf(0.83));

            assert_relative_eq!(ean(0.32)?.otu_rate_at(depth), expected, epsilon = 1e-9);

            Ok(())
        }
    }

    mod best_mix {
        use super::*;

        #[test]
        fn at_30m_1_4_bar() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(30.0);
            let ppo2_max = Bar::new(1.4);
            let expected_fo2 = ppo2_max / (depth / env.water_density() + env.surface_pressure());
            let best = EANx::best_mix(depth, ppo2_max, env)?;

            assert_relative_eq!(f64::from(best.fo2()), expected_fo2.min(1.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn ppo2_at_target_equals_limit() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let depth = Meters::new(40.0);
            let ppo2_max = Bar::new(1.4);
            let best = EANx::best_mix(depth, ppo2_max, env)?;

            assert_relative_eq!(best.ppo2_at(depth).pressure(), ppo2_max, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn shallow_depth_clamps_to_pure_o2() -> Result<(), InvalidEANxError> {
            // fo2 = 1.4 / (3/9.948 + 1.013) ≈ 1.065 > 1.0 → clamps to 1.0
            let best =
                EANx::best_mix(Meters::new(3.0), Bar::new(1.4), DiveEnvironment::standard())?;

            assert_relative_eq!(f64::from(best.fo2()), 1.0, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn very_deep_returns_err() {
            // At extreme depth, required fo2 would be below 10% minimum
            assert!(
                EANx::best_mix(
                    Meters::new(200.0),
                    Bar::new(1.4),
                    DiveEnvironment::standard()
                )
                .is_err()
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_air() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.21)?.to_string(), "Air");
            assert_eq!(ean(0.215)?.to_string(), "Air");
            assert_ne!(ean(0.205)?.to_string(), "Air");

            Ok(())
        }

        #[test]
        fn display_named_nitrox_mixes() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.28)?.to_string(), "EANx 28");
            assert_eq!(ean(0.30)?.to_string(), "EANx 30");
            assert_eq!(ean(0.32)?.to_string(), "EANx 32");
            assert_eq!(ean(0.36)?.to_string(), "EANx 36");
            assert_eq!(ean(0.40)?.to_string(), "EANx 40");

            Ok(())
        }

        #[test]
        fn display_high_o2_mixes() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.50)?.to_string(), "O₂ 50%");
            assert_eq!(ean(0.80)?.to_string(), "O₂ 80%");

            Ok(())
        }

        #[test]
        fn display_hypoxic_mixes() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.10)?.to_string(), "Hypoxic 10");
            assert_eq!(ean(0.12)?.to_string(), "Hypoxic 12");
            assert_eq!(ean(0.14)?.to_string(), "Hypoxic 14");
            assert_eq!(ean(0.16)?.to_string(), "Hypoxic 16");
            assert_eq!(ean(0.18)?.to_string(), "Hypoxic 18");

            Ok(())
        }

        #[test]
        fn display_pure_o2() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(1.0)?.to_string(), "Pure O₂");

            Ok(())
        }

        #[test]
        fn display_unnamed_mix_shows_fraction() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.25)?.to_string(), "25%");
            assert_eq!(ean(0.33)?.to_string(), "33%");

            Ok(())
        }

        #[test]
        fn display_is_blend_method_agnostic() -> Result<(), InvalidEANxError> {
            assert_eq!(ean(0.32)?.to_string(), ean_psa(0.32)?.to_string());

            Ok(())
        }
    }

    mod try_from_percent {
        use super::*;

        #[rstest]
        #[case(0.21)]
        #[case(0.32)]
        #[case(0.40)]
        #[case(1.0)]
        fn try_from_percent_preserves_fraction(
            #[case] fraction: f64,
        ) -> Result<(), InvalidEANxError> {
            let pct = Percent::new(fraction)?;

            assert_eq!(EANx::try_from(pct)?.fo2(), pct);

            Ok(())
        }

        #[test]
        fn try_from_percent_rejects_below_minimum() -> Result<(), InvalidEANxError> {
            assert!(EANx::try_from(Percent::new(0.09)?).is_err());

            Ok(())
        }

        #[test]
        fn try_from_percent_accepts_fraction_that_rounds_into_valid_range()
        -> Result<(), InvalidEANxError> {
            assert!(EANx::try_from(Percent::new(0.316)?).is_ok());

            Ok(())
        }
    }

    mod default {
        use super::*;

        #[rstest]
        fn default_is_air() {
            assert_eq!(EANx::default().to_string(), "Air");
        }

        #[rstest]
        fn default_fo2_is_21_percent() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(EANx::default().fo2(), Percent::new(0.21)?);

            Ok(())
        }
    }
}
