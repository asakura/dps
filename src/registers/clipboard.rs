//! OS clipboard integration for the register system.

use super::RegisterValue;
use super::store::RegisterStore;

impl RegisterStore {
    /// Writes `value` to the OS clipboard; errors are silently discarded.
    pub(in crate::registers) fn write_os(value: RegisterValue) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(value.to_string());
        }
    }

    /// Reads a [`RegisterValue`] from the OS clipboard.
    ///
    /// Returns `None` if the clipboard is unavailable or contains unrecognised text.
    pub(in crate::registers) fn read_os() -> Option<RegisterValue> {
        let text = arboard::Clipboard::new().ok()?.get_text().ok()?;
        text.parse().ok()
    }
}
