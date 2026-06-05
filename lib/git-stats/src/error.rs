//! Error hierarchy for the `dps-git-stats` binary.
//!
//! All fallible operations surface one of the top-level variants of [`Error`].
//! Each sub-domain owns a dedicated error type; git and stats errors are
//! heap-allocated in the top-level enum so `Error` stays pointer-sized.
//!
//! ```
//! use dps_git_stats::error::{Error, GitError};
//! let git_err = GitError::RevSpec("unknown ref 'typo'".into());
//! let err: Error = git_err.into();
//! assert!(err.to_string().starts_with("git:"));
//! ```

/// Repository-layer errors produced by [`crate::git`].
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// `gix` failed to open the repository.
    #[error("failed to open repository: {0}")]
    Open(#[from] gix::open::Error),

    /// Setting up the revision walk failed.
    #[error("revision walk setup failed: {0}")]
    Walk(#[from] gix::revision::walk::Error),

    /// Iterating commits during the revision walk failed.
    #[error("commit iteration failed: {0}")]
    WalkIter(#[from] gix::revision::walk::iter::Error),

    /// Reading a single object from the ODB failed.
    #[error("failed to read object: {0}")]
    Object(#[from] gix::object::find::existing::Error),

    /// Decoding a commit's fields (author, tree id, …) failed.
    #[error("failed to decode commit: {0}")]
    Decode(#[from] gix::objs::decode::Error),

    /// HEAD reference could not be resolved.
    #[error("could not resolve HEAD: {0}")]
    Head(#[from] gix::reference::head_commit::Error),

    /// A revision specifier could not be parsed.
    #[error("could not parse revision '{0}'")]
    RevSpec(String),

    /// An object turned out to be a different kind than expected (e.g. a blob
    /// where a commit was required).
    #[error("unexpected object kind: {0}")]
    ObjectKind(String),

    /// Computing the diff between two trees failed.
    #[error("tree diff failed: {0}")]
    TreeDiff(String),

    /// Accessing a commit's tree object failed.
    #[error("could not read commit tree: {0}")]
    Tree(String),
}

/// Statistics-layer errors produced by [`crate::stats`].
#[derive(Debug, thiserror::Error)]
pub enum StatsError {
    /// Polars returned an unexpected error.
    #[error("polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// The data frame was missing an expected column.
    #[error("missing column '{0}'")]
    MissingColumn(String),
}

/// Top-level error type for the binary.
///
/// Git and stats errors are heap-allocated so `Error` stays pointer-sized.
/// IO errors are stored directly — `std::io::Error` is already pointer-sized.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A git operation failed.
    #[error("git: {0}")]
    Git(Box<GitError>),

    /// A statistics computation failed.
    #[error("stats: {0}")]
    Stats(Box<StatsError>),

    /// An IO error occurred while writing output.
    #[error("io: {0}")]
    Render(std::io::Error),
}

impl From<GitError> for Error {
    fn from(e: GitError) -> Self {
        Self::Git(Box::new(e))
    }
}

impl From<StatsError> for Error {
    fn from(e: StatsError) -> Self {
        Self::Stats(Box::new(e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Render(e)
    }
}
