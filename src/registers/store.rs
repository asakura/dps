use std::collections::{HashMap, VecDeque};

use super::RegisterValue;

/// Maximum number of entries retained in the yank ring.
const YANK_RING_CAP: usize = 9;

/// Vim-style named register store.
///
/// Owns a map from register characters to [`RegisterValue`]s. Use
/// [`push_yank`](RegisterStore::push_yank) and
/// [`push_delete`](RegisterStore::push_delete) for the high-level operations,
/// or [`write`](RegisterStore::write) / [`read`](RegisterStore::read) for
/// direct access.
///
/// # Storage design
///
/// `slots` is a [`HashMap`] rather than a fixed array because the usable
/// register key space is small and **sparse**: at most ~38 distinct chars
/// (`'"'`, `'0'`–`'9'`, `'a'`–`'z'`, `'+'`, `'*'`) and only a subset of
/// those are ever written in a typical session. Allocating a
/// `[Option<RegisterValue>; 128]` array upfront would waste memory for
/// registers that are never touched, and would require every access to
/// bounds-check the char. A `HashMap` pays per-entry rather than per-slot,
/// so memory scales with actual usage.
///
/// The performance trade-off (hashing overhead vs. direct array indexing) is
/// irrelevant here: the store is accessed a handful of times per user
/// keystroke, far below any throughput threshold where the difference would
/// be measurable.
///
/// ## Yank ring
///
/// Each call to [`push_yank`](RegisterStore::push_yank) prepends to an internal
/// `VecDeque` capped at [`YANK_RING_CAP`] entries. Reading register `'0'`
/// always returns the head of the ring — the most recent yank — so the
/// observable behaviour of `'0'` is unchanged from a flat-slot model. The
/// ring preserves older yanks in insertion order, ready for a future
/// paste-cycling operation.
#[derive(Debug, Default)]
pub struct RegisterStore {
    slots: HashMap<char, RegisterValue>,
    yank_ring: VecDeque<RegisterValue>,
}

impl RegisterStore {
    /// Writes `value` to register `reg`, following Vim register semantics.
    ///
    /// - `'_'` (black hole): silently discarded.
    /// - `'A'`–`'Z'` (append): writes to the lowercase partner (`'A'` → `'a'`);
    ///   for typed domain values last-write wins rather than concatenating.
    /// - `'+'` / `'*'` (OS clipboard): writes to the OS clipboard and mirrors to `'"'`.
    /// - All other characters: writes to `reg` and mirrors to `'"'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    ///
    /// // Regular write goes to 'a' and is mirrored to the unnamed register.
    /// store.write('a', RegisterValue::EANx(ean32));
    /// assert!(store.read('a').is_some());
    /// assert!(store.read('"').is_some());
    ///
    /// // Black-hole register silently discards.
    /// store.write('_', RegisterValue::EANx(ean32));
    /// assert!(store.read('_').is_none());
    ///
    /// // Uppercase redirects to the lowercase partner.
    /// store.write('B', RegisterValue::EANx(ean32));
    /// assert!(store.read('b').is_some());
    /// assert!(store.read('B').is_none());
    /// ```
    pub fn write(&mut self, reg: char, value: RegisterValue) {
        match reg {
            '_' => {} // black hole — discard
            'A'..='Z' => {
                let lower = char::from(reg as u8 + 32);
                let _ = self.slots.insert(lower, value);
                let _ = self.slots.insert('"', value);
                Self::write_os(value);
            }
            '+' | '*' => {
                let _ = self.slots.insert('"', value);
                Self::write_os(value);
            }
            _ => {
                let _ = self.slots.insert(reg, value);
                let _ = self.slots.insert('"', value);
                Self::write_os(value);
            }
        }
    }

    /// Records a yank operation.
    ///
    /// Writes to `reg` (defaulting to the unnamed register `'"'` when `None`),
    /// and prepends `value` to the internal yank ring. Register `'0'` always
    /// reads as the ring head (most recent yank); older entries are retained up
    /// to [`YANK_RING_CAP`] for future paste-cycling.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    ///
    /// // Without a target register, the value lands in '"' and '0'.
    /// store.push_yank(None, RegisterValue::EANx(ean32));
    /// assert!(store.read('"').is_some());
    /// assert!(store.read('0').is_some());
    ///
    /// // With an explicit register, the value also lands in '0'.
    /// store.push_yank(Some('a'), RegisterValue::EANx(ean32));
    /// assert!(store.read('a').is_some());
    /// assert!(store.read('0').is_some());
    /// ```
    pub fn push_yank(&mut self, reg: Option<char>, value: RegisterValue) {
        let target = reg.unwrap_or('"');
        self.write(target, value);
        self.yank_ring.push_front(value);
        self.yank_ring.truncate(YANK_RING_CAP);
    }

    /// Records a delete operation.
    ///
    /// Shifts the delete-history stack: `'1'`→`'2'`, …, `'8'`→`'9'`. Writes
    /// the new value to `'1'` and mirrors it to the unnamed register `'"'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let ean36 = EANx::try_from(Percent::new(0.36).unwrap()).unwrap();
    ///
    /// store.push_delete(RegisterValue::EANx(ean32));
    /// assert!(store.read('1').is_some());
    /// assert!(store.read('"').is_some());
    ///
    /// // A second delete shifts the first value from '1' to '2'.
    /// store.push_delete(RegisterValue::EANx(ean36));
    /// assert_eq!(store.read('1'), Some(RegisterValue::EANx(ean36)));
    /// assert_eq!(store.read('2'), Some(RegisterValue::EANx(ean32)));
    /// ```
    pub fn push_delete(&mut self, value: RegisterValue) {
        for n in (b'2'..=b'9').rev() {
            let from = char::from(n - 1);
            let to = char::from(n);

            if let Some(v) = self.slots.get(&from).copied() {
                let _ = self.slots.insert(to, v);
            } else {
                self.slots.remove(&to);
            }
        }

        let _ = self.slots.insert('1', value);
        let _ = self.slots.insert('"', value);
    }

    /// Reads the value stored in register `reg`.
    ///
    /// - `'_'` (black hole): always `None`.
    /// - `'+'` / `'*'` (OS clipboard): reads from the OS clipboard.
    /// - `'0'` (yank register): returns the head of the internal yank ring —
    ///   the most recent value passed to [`push_yank`](RegisterStore::push_yank).
    /// - All other characters: reads from the in-memory slot.
    ///
    /// Returns `None` if the register has never been written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::{RegisterStore, RegisterValue};
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    ///
    /// let mut store = RegisterStore::default();
    /// assert!(store.read('a').is_none()); // never written
    /// assert!(store.read('_').is_none()); // black hole
    ///
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// store.write('a', RegisterValue::EANx(ean32));
    /// assert_eq!(store.read('a'), Some(RegisterValue::EANx(ean32)));
    /// ```
    #[must_use]
    pub fn read(&self, reg: char) -> Option<RegisterValue> {
        match reg {
            '_' => None,
            '+' | '*' => Self::read_os(),
            '0' => self.yank_ring.front().copied(),
            _ => self.slots.get(&reg).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::{Report, eyre};
    use rstest::{fixture, rstest};

    use super::{RegisterStore, RegisterValue};
    use crate::gas::EANx;
    use crate::units::Percent;

    #[fixture]
    fn ean32() -> Result<EANx, Report> {
        let pct = Percent::new(0.32).ok_or_else(|| eyre!("invalid fraction"))?;

        Ok(EANx::try_from(pct)?)
    }

    #[fixture]
    fn ean36() -> Result<EANx, Report> {
        let pct = Percent::new(0.36).ok_or_else(|| eyre!("invalid fraction"))?;

        Ok(EANx::try_from(pct)?)
    }

    mod write {
        use super::*;

        #[rstest]
        fn regular_reg_stores_value(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.write('a', RegisterValue::EANx(ean32?));

            assert!(matches!(store.read('a'), Some(RegisterValue::EANx(_))));

            Ok(())
        }

        #[rstest]
        fn regular_reg_mirrors_to_unnamed(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.write('a', RegisterValue::EANx(ean32?));

            assert!(store.read('"').is_some());

            Ok(())
        }

        #[rstest]
        fn black_hole_discards(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.write('_', RegisterValue::EANx(ean32?));

            assert!(store.read('_').is_none());

            Ok(())
        }

        #[rstest]
        fn uppercase_redirects_to_lowercase(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.write('A', RegisterValue::EANx(ean32?));

            assert!(store.read('a').is_some());
            assert!(store.read('A').is_none());

            Ok(())
        }

        #[rstest]
        fn uppercase_mirrors_to_unnamed(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.write('A', RegisterValue::EANx(ean32?));

            assert!(store.read('"').is_some());

            Ok(())
        }
    }

    mod push_yank {
        use super::*;
        use super::super::YANK_RING_CAP;

        #[rstest]
        fn none_writes_to_unnamed_and_yank(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.push_yank(None, RegisterValue::EANx(ean32?));

            assert!(store.read('"').is_some());
            assert!(store.read('0').is_some());

            Ok(())
        }

        #[rstest]
        fn named_reg_writes_to_reg_and_yank(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.push_yank(Some('a'), RegisterValue::EANx(ean32?));

            assert!(store.read('a').is_some());
            assert!(store.read('0').is_some());

            Ok(())
        }

        #[rstest]
        fn yank_zero_returns_most_recent(
            ean32: Result<EANx, Report>,
            ean36: Result<EANx, Report>,
        ) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v1 = RegisterValue::EANx(ean32?);
            let v2 = RegisterValue::EANx(ean36?);

            store.push_yank(None, v1);
            store.push_yank(None, v2);

            assert_eq!(store.read('0'), Some(v2));

            Ok(())
        }

        #[rstest]
        fn ring_retains_history_up_to_cap(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            for _ in 0..=YANK_RING_CAP {
                store.push_yank(None, v);
            }

            assert_eq!(store.yank_ring.len(), YANK_RING_CAP);

            Ok(())
        }
    }

    mod push_delete {
        use super::*;

        #[rstest]
        fn first_delete_writes_to_1_and_unnamed(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            store.push_delete(RegisterValue::EANx(ean32?));

            assert!(store.read('1').is_some());
            assert!(store.read('"').is_some());

            Ok(())
        }

        #[rstest]
        fn second_delete_shifts_stack(
            ean32: Result<EANx, Report>,
            ean36: Result<EANx, Report>,
        ) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v1 = RegisterValue::EANx(ean32?);
            let v2 = RegisterValue::EANx(ean36?);

            store.push_delete(v1);
            store.push_delete(v2);

            assert_eq!(store.read('1'), Some(v2));
            assert_eq!(store.read('2'), Some(v1));

            Ok(())
        }

        #[rstest]
        fn full_stack_stays_within_9(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            for _ in 0..10 {
                store.push_delete(v);
            }

            assert!(('1'..='9').all(|c| store.read(c).is_some()));

            Ok(())
        }

        #[rstest]
        fn gap_in_stack_clears_higher_slot(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            store.push_delete(v);
            assert!(store.read('2').is_none());

            store.push_delete(v);
            assert!(store.read('2').is_some());
            assert!(store.read('3').is_none());

            Ok(())
        }
    }

    mod read {
        use super::*;

        #[test]
        fn unwritten_register_is_none() {
            let store = RegisterStore::default();
            assert!(store.read('a').is_none());
        }

        #[test]
        fn black_hole_is_always_none() {
            let store = RegisterStore::default();
            assert!(store.read('_').is_none());
        }

        #[rstest]
        fn after_write_returns_value(ean32: Result<EANx, Report>) -> Result<(), Report> {
            let mut store = RegisterStore::default();
            let v = RegisterValue::EANx(ean32?);

            store.write('a', v);
            assert_eq!(store.read('a'), Some(v));

            Ok(())
        }
    }
}
