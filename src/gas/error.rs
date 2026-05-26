//! Gas domain error type.

/// Error from a gas-domain operation.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    /// `EANx` blend validation failed.
    #[error(transparent)]
    InvalidEANx(#[from] super::InvalidEANxError),
    /// Membrane diluent fraction validation failed.
    #[error(transparent)]
    InvalidMembrane(#[from] super::InvalidMembraneFractionsError),
}
