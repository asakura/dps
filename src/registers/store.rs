//! Concrete [`RegisterStore`] implementation.
//!
//! ```
//! use dps::registers::{RegisterName, RegisterStore, RegisterValue};
//! use dps::gas::EANx;
//! use dps::units::Percent;
//!
//! let mut store = RegisterStore::default();
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//!
//! store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32));
//! assert_eq!(store.read(RegisterName::Yank), Some(RegisterValue::EANx(ean32)));
//! ```

use super::name::RegIndex;
use super::{RegisterError, RegisterName, RegisterValue, YankRingTooSmall};

use std::collections::{HashMap, VecDeque};

/// Maximum number of entries retained in the yank ring.
const YANK_RING_CAP: usize = 9;

/// Vim-style named register store.
///
/// Owns a map from [`RegisterName`] to [`RegisterValue`]s. Use
/// [`push_yank`](RegisterStore::push_yank) and
/// [`push_delete`](RegisterStore::push_delete) for the high-level operations,
/// or [`write`](RegisterStore::write) / [`read`](RegisterStore::read) for
/// direct access.
///
/// # Storage design
///
/// `slots` is a [`HashMap`] rather than a fixed array because the usable
/// register key space is small and **sparse**: at most ~38 distinct
/// [`RegisterName`] keys and only a subset of those are ever written in a
/// typical session. A `HashMap` pays per-entry rather than per-slot, so
/// memory scales with actual usage.
///
/// The performance trade-off (hashing overhead vs. direct array indexing) is
/// irrelevant here: the store is accessed a handful of times per user
/// keystroke, far below any throughput threshold where the difference would
/// be measurable.
///
/// ## Yank ring
///
/// Each call to [`push_yank`](RegisterStore::push_yank) prepends to an internal
/// `VecDeque` capped at `YANK_RING_CAP` entries and resets the ring cursor
/// to 0. Reading [`RegisterName::Yank`] always returns the ring head. Older entries
/// drive paste cycling: [`cycle_yank`](RegisterStore::cycle_yank) advances the
/// cursor so components can replace the most recently pasted row with the next
/// ring entry.
#[derive(Debug, Default)]
pub struct RegisterStore {
    slots: HashMap<RegisterName, RegisterValue>,
    yank_ring: VecDeque<RegisterValue>,
    ring_cursor: usize,
}

impl RegisterStore {
    /// Writes `value` to register `reg`, following Vim register semantics.
    ///
    /// - [`BlackHole`](RegisterName::BlackHole): silently discarded.
    /// - [`Named('A'..='Z')`](RegisterName::Named) (append): writes to the lowercase partner;
    ///   for typed domain values last-write wins rather than concatenating.
    /// - [`Clipboard`](RegisterName::Clipboard) / [`Selection`](RegisterName::Selection) (OS clipboard):
    ///   writes to the OS clipboard and mirrors to the unnamed register.
    /// - [`Numbered`](RegisterName::Numbered): writes only to the numbered slot; no mirroring
    ///   to the unnamed register and no OS clipboard interaction. Use
    ///   [`push_delete`](RegisterStore::push_delete) to perform a full delete with stack-shift.
    /// - All other registers: writes to `reg` and mirrors to the unnamed register.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let reg_a = RegisterName::try_from('a').unwrap();
    /// let reg_b_upper = RegisterName::try_from('B').unwrap();
    /// let reg_black_hole = RegisterName::BlackHole;
    ///
    /// // Regular write goes to 'a' and is mirrored to the unnamed register.
    /// store.write(reg_a, RegisterValue::EANx(ean32));
    /// assert!(store.read(reg_a).is_some());
    /// assert!(store.read(RegisterName::Unnamed).is_some());
    ///
    /// // Black-hole register silently discards.
    /// store.write(reg_black_hole, RegisterValue::EANx(ean32));
    /// assert!(store.read(reg_black_hole).is_none());
    ///
    /// // Uppercase redirects to the lowercase partner.
    /// store.write(reg_b_upper, RegisterValue::EANx(ean32));
    /// assert!(store.read(RegisterName::try_from('b').unwrap()).is_some());
    /// assert!(store.read(reg_b_upper).is_none());
    /// ```
    pub fn write(&mut self, reg: RegisterName, value: RegisterValue) {
        match reg {
            RegisterName::BlackHole => {}
            RegisterName::Named(rc) if char::from(rc).is_ascii_uppercase() => {
                let Ok(lower) = RegisterName::try_from(char::from(rc).to_ascii_lowercase()) else {
                    unreachable!()
                };

                self.slots.insert(lower, value);
                self.slots.insert(RegisterName::Unnamed, value);

                Self::write_os(value);
            }
            RegisterName::Clipboard | RegisterName::Selection => {
                self.slots.insert(RegisterName::Unnamed, value);

                Self::write_os(value);
            }
            RegisterName::Numbered(_) => {
                self.slots.insert(reg, value);
            }
            reg => {
                self.slots.insert(reg, value);
                self.slots.insert(RegisterName::Unnamed, value);

                Self::write_os(value);
            }
        }
    }

    /// Records a yank operation.
    ///
    /// Writes to `reg` and prepends `value` to the internal yank ring. Register
    /// `'0'` always reads as the ring head (most recent yank); older entries are
    /// retained up to `YANK_RING_CAP` entries for future paste-cycling. Pass
    /// [`RegisterName::Unnamed`] to target the unnamed register `'"'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    ///
    /// // Unnamed register: value lands in '"' and '0'.
    /// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32));
    /// assert!(store.read(RegisterName::Unnamed).is_some());
    /// assert!(store.read(RegisterName::Yank).is_some());
    ///
    /// // Named register: value also lands in '0'.
    /// let reg_a = RegisterName::try_from('a').unwrap();
    /// store.push_yank(reg_a, RegisterValue::EANx(ean32));
    /// assert!(store.read(reg_a).is_some());
    /// assert!(store.read(RegisterName::Yank).is_some());
    /// ```
    ///
    pub fn push_yank(&mut self, reg: RegisterName, value: RegisterValue) {
        self.write(reg, value);
        self.yank_ring.push_front(value);
        self.yank_ring.truncate(YANK_RING_CAP);
        self.ring_cursor = 0;
    }

    /// Advances the yank-ring cursor and returns the entry at the new position.
    ///
    /// Returns `None` when the ring has fewer than two entries (nothing to
    /// cycle to). The cursor wraps on overflow, so successive calls cycle
    /// indefinitely through the ring.
    ///
    /// Called by components in response to [`EditOp::CyclePaste`](crate::action::EditOp).
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let ean36 = EANx::try_from(Percent::new(0.36).unwrap()).unwrap();
    ///
    /// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32)); // ring = [ean32]
    /// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean36)); // ring = [ean36, ean32]
    ///
    /// // First cycle: advance to ring[1] (older yank).
    /// assert_eq!(store.cycle_yank(), Ok(RegisterValue::EANx(ean32)));
    /// // Second cycle: wraps back to ring[0].
    /// assert_eq!(store.cycle_yank(), Ok(RegisterValue::EANx(ean36)));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RegisterError::YankRingTooSmall`] when the ring has fewer than two entries.
    pub fn cycle_yank(&mut self) -> Result<RegisterValue, RegisterError> {
        if self.yank_ring.len() < 2 {
            return Err(YankRingTooSmall.into());
        }

        self.ring_cursor = (self.ring_cursor + 1) % self.yank_ring.len();
        self.yank_ring
            .get(self.ring_cursor)
            .copied()
            .ok_or_else(|| YankRingTooSmall.into())
    }

    /// Resets the yank-ring cursor to 0 (the most recent yank).
    ///
    /// Components call this when a fresh paste or any non-cycle action breaks
    /// the current paste-cycling chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let ean36 = EANx::try_from(Percent::new(0.36).unwrap()).unwrap();
    ///
    /// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32));
    /// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean36));
    /// store.cycle_yank().unwrap(); // cursor = 1
    ///
    /// store.reset_ring_cursor();
    /// // Next cycle_yank now starts from ring[0] again.
    /// assert_eq!(store.cycle_yank(), Ok(RegisterValue::EANx(ean32)));
    /// ```
    pub const fn reset_ring_cursor(&mut self) {
        self.ring_cursor = 0;
    }

    /// Records a delete operation.
    ///
    /// Shifts the delete-history stack: `'1'`→`'2'`, …, `'8'`→`'9'`. Writes
    /// the new value to `'1'` and mirrors it to the unnamed register `'"'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let ean36 = EANx::try_from(Percent::new(0.36).unwrap()).unwrap();
    ///
    /// store.push_delete(RegisterValue::EANx(ean32));
    /// assert!(store.read(RegisterName::try_from('1').unwrap()).is_some());
    /// assert!(store.read(RegisterName::Unnamed).is_some());
    ///
    /// // A second delete shifts the first value from '1' to '2'.
    /// store.push_delete(RegisterValue::EANx(ean36));
    /// assert_eq!(store.read(RegisterName::try_from('1').unwrap()), Some(RegisterValue::EANx(ean36)));
    /// assert_eq!(store.read(RegisterName::try_from('2').unwrap()), Some(RegisterValue::EANx(ean32)));
    /// ```
    pub fn push_delete(&mut self, value: RegisterValue) {
        for n in (1u8..=8).rev() {
            let from = RegisterName::Numbered(RegIndex(n));

            if let Some(to) = from.next_numbered() {
                if let Some(v) = self.slots.get(&from).copied() {
                    self.slots.insert(to, v);
                } else {
                    self.slots.remove(&to);
                }
            }
        }

        self.slots
            .insert(RegisterName::Numbered(RegIndex(1)), value);
        self.slots.insert(RegisterName::Unnamed, value);
    }

    /// Reads the value stored in register `reg`.
    ///
    /// - [`BlackHole`](RegisterName::BlackHole): always `None`.
    /// - [`Clipboard`](RegisterName::Clipboard) / [`Selection`](RegisterName::Selection)
    ///   (OS clipboard): reads from the OS clipboard.
    /// - [`Yank`](RegisterName::Yank) (yank register): returns the head of
    ///   the internal yank ring — the most recent value passed to
    ///   [`push_yank`](RegisterStore::push_yank).
    /// - All other registers: reads from the in-memory slot.
    ///
    /// Returns `None` if the register has never been written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterName, RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let reg_a = RegisterName::try_from('a').unwrap();
    /// let reg_black_hole = RegisterName::BlackHole;
    ///
    /// assert!(store.read(reg_a).is_none());         // never written
    /// assert!(store.read(reg_black_hole).is_none()); // black hole
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// store.write(reg_a, RegisterValue::EANx(ean32));
    /// assert_eq!(store.read(reg_a), Some(RegisterValue::EANx(ean32)));
    /// ```
    #[must_use]
    pub fn read(&self, reg: RegisterName) -> Option<RegisterValue> {
        match reg {
            RegisterName::BlackHole => None,
            RegisterName::Clipboard | RegisterName::Selection => Self::read_os(),
            RegisterName::Yank => self.yank_ring.front().copied(),
            reg => self.slots.get(&reg).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::gas::{EANx, InvalidEANxError};
    use crate::registers::{RegisterError, RegisterName, name::RegIndex};
    use crate::units::Percent;

    use rstest::{fixture, rstest};

    use core::assert_matches;

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error(transparent)]
        Register(#[from] RegisterError),
        #[error(transparent)]
        EANx(#[from] InvalidEANxError),
    }

    fn reg(c: char) -> Result<RegisterName, RegisterError> {
        RegisterName::try_from(c)
    }

    #[fixture]
    fn ean32() -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(0.32)?;

        EANx::try_from(pct)
    }

    #[fixture]
    fn ean36() -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(0.36)?;

        EANx::try_from(pct)
    }

    mod write {
        use super::*;

        #[rstest]
        fn regular_reg_stores_value(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            store.write(reg('a')?, RegisterValue::EANx(ean32?));

            assert_matches!(store.read(reg('a')?), Some(RegisterValue::EANx(_)));

            Ok(())
        }

        #[rstest]
        fn regular_reg_mirrors_to_unnamed(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();

            store.write(reg('a')?, RegisterValue::EANx(ean32?));

            assert!(store.read(RegisterName::Unnamed).is_some());

            Ok(())
        }

        #[rstest]
        fn black_hole_discards(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();

            store.write(RegisterName::BlackHole, RegisterValue::EANx(ean32?));

            assert!(store.read(RegisterName::BlackHole).is_none());

            Ok(())
        }

        #[rstest]
        fn uppercase_redirects_to_lowercase(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();

            store.write(reg('A')?, RegisterValue::EANx(ean32?));

            assert!(store.read(reg('a')?).is_some());
            assert!(store.read(reg('A')?).is_none());

            Ok(())
        }

        #[rstest]
        fn uppercase_mirrors_to_unnamed(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();

            store.write(reg('A')?, RegisterValue::EANx(ean32?));

            assert!(store.read(RegisterName::Unnamed).is_some());

            Ok(())
        }

        #[rstest]
        fn numbered_writes_only_to_slot(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();

            store.write(
                RegisterName::Numbered(RegIndex(3)),
                RegisterValue::EANx(ean32?),
            );

            assert!(store.read(RegisterName::Numbered(RegIndex(3))).is_some());
            assert!(store.read(RegisterName::Unnamed).is_none());

            Ok(())
        }
    }

    mod push_yank {
        use super::*;

        #[rstest]
        fn unnamed_writes_to_unnamed_and_yank(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();
            store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32?));

            assert!(store.read(RegisterName::Unnamed).is_some());
            assert!(store.read(RegisterName::Yank).is_some());

            Ok(())
        }

        #[rstest]
        fn named_reg_writes_to_reg_and_yank(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            store.push_yank(reg('a')?, RegisterValue::EANx(ean32?));

            assert!(store.read(reg('a')?).is_some());
            assert!(store.read(RegisterName::Yank).is_some());

            Ok(())
        }

        #[rstest]
        fn yank_zero_returns_most_recent(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();
            let v1 = RegisterValue::EANx(ean32?);
            let v2 = RegisterValue::EANx(ean36?);

            store.push_yank(RegisterName::Unnamed, v1);
            store.push_yank(RegisterName::Unnamed, v2);

            assert_eq!(store.read(RegisterName::Yank), Some(v2));

            Ok(())
        }

        #[rstest]
        fn ring_retains_history_up_to_cap(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            for _ in 0..=YANK_RING_CAP {
                store.push_yank(RegisterName::Unnamed, v);
            }

            assert_eq!(store.yank_ring.len(), YANK_RING_CAP);

            Ok(())
        }

        #[rstest]
        fn resets_ring_cursor(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let v1 = RegisterValue::EANx(ean32?);
            let v2 = RegisterValue::EANx(ean36?);

            store.push_yank(RegisterName::Unnamed, v1);
            store.push_yank(RegisterName::Unnamed, v2);
            store.cycle_yank()?; // cursor = 1
            store.push_yank(RegisterName::Unnamed, v1);

            assert_eq!(store.ring_cursor, 0);

            Ok(())
        }
    }

    mod cycle_yank {
        use super::*;

        #[rstest]
        fn returns_err_for_empty_ring() {
            let mut store = RegisterStore::default();
            assert!(store.cycle_yank().is_err());
        }

        #[rstest]
        fn returns_err_for_single_entry(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();
            store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32?));

            assert!(store.cycle_yank().is_err());

            Ok(())
        }

        #[rstest]
        fn advances_to_older_yank(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), InvalidEANxError> {
            let mut store = RegisterStore::default();
            let older = RegisterValue::EANx(ean32?);
            let newer = RegisterValue::EANx(ean36?);

            store.push_yank(RegisterName::Unnamed, older); // ring = [older]
            store.push_yank(RegisterName::Unnamed, newer); // ring = [newer, older], cursor = 0

            assert_eq!(store.cycle_yank(), Ok(older)); // cursor = 1

            Ok(())
        }

        #[rstest]
        fn wraps_back_to_newest(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let older = RegisterValue::EANx(ean32?);
            let newer = RegisterValue::EANx(ean36?);

            store.push_yank(RegisterName::Unnamed, older);
            store.push_yank(RegisterName::Unnamed, newer);
            store.cycle_yank()?; // cursor = 1 → older

            assert_eq!(store.cycle_yank(), Ok(newer)); // wraps to cursor = 0 → newer

            Ok(())
        }

        #[rstest]
        fn reset_restarts_cycle(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let older = RegisterValue::EANx(ean32?);
            let newer = RegisterValue::EANx(ean36?);

            store.push_yank(RegisterName::Unnamed, older);
            store.push_yank(RegisterName::Unnamed, newer);
            store.cycle_yank()?; // cursor = 1
            store.reset_ring_cursor(); // back to 0

            assert_eq!(store.cycle_yank(), Ok(older)); // cursor = 1 again

            Ok(())
        }
    }

    mod push_delete {
        use super::*;

        #[rstest]
        fn first_delete_writes_to_1_and_unnamed(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            store.push_delete(RegisterValue::EANx(ean32?));

            assert!(store.read(reg('1')?).is_some());
            assert!(store.read(RegisterName::Unnamed).is_some());

            Ok(())
        }

        #[rstest]
        fn second_delete_shifts_stack(
            ean32: Result<EANx, InvalidEANxError>,
            ean36: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let v1 = RegisterValue::EANx(ean32?);
            let v2 = RegisterValue::EANx(ean36?);

            store.push_delete(v1);
            store.push_delete(v2);

            assert_eq!(store.read(reg('1')?), Some(v2));
            assert_eq!(store.read(reg('2')?), Some(v1));

            Ok(())
        }

        #[rstest]
        fn full_stack_stays_within_9(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            for _ in 0..10 {
                store.push_delete(v);
            }

            for c in '1'..='9' {
                let r = reg(c)?;
                assert!(
                    store.read(r).is_some(),
                    "register {c:?} should have a value"
                );
            }

            Ok(())
        }

        #[rstest]
        fn gap_in_stack_clears_higher_slot(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            store.push_delete(v);
            assert!(store.read(reg('2')?).is_none());

            store.push_delete(v);
            assert!(store.read(reg('2')?).is_some());
            assert!(store.read(reg('3')?).is_none());

            Ok(())
        }
    }

    mod read {
        use super::*;

        #[rstest]
        fn unwritten_register_is_none() -> Result<(), RegisterError> {
            let store = RegisterStore::default();
            assert!(store.read(reg('a')?).is_none());

            Ok(())
        }

        #[rstest]
        fn black_hole_is_always_none() {
            let store = RegisterStore::default();
            assert!(store.read(RegisterName::BlackHole).is_none());
        }

        #[rstest]
        fn after_write_returns_value(
            ean32: Result<EANx, InvalidEANxError>,
        ) -> Result<(), TestError> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            store.write(reg('a')?, v);
            assert_eq!(store.read(reg('a')?), Some(v));

            Ok(())
        }
    }
}
