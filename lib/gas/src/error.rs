//! Gas domain error type.

/// Error from a gas-domain operation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// `EANx` blend validation failed.
    #[error(transparent)]
    InvalidEANx(#[from] crate::eanx::InvalidEANxError),
    /// Membrane diluent fraction validation failed.
    #[error(transparent)]
    InvalidMembrane(#[from] crate::blend::InvalidMembraneFractionsError),
}
