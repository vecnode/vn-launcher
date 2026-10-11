//! Error type for the pure core.
//!
//! Every variant is something a person using the launcher could be told. Nothing
//! here is an internal invariant: the core is written so that malformed input
//! (a hostile release payload, a hand-edited catalog) is a value, not a panic.

use std::fmt;

/// What went wrong in the pure core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A repository reference was not in the form `owner/name`.
    BadRepo(String),
    /// A name inside `owner/name` was not a legal GitHub name.
    BadRepoPart {
        /// Whether it was the owner or the repository name.
        part: &'static str,
        /// The value that was rejected.
        value: String,
    },
    /// A release payload was not the JSON the GitHub API returns.
    ReleasePayload(String),
    /// A catalog document was not valid TOML, or did not match the schema.
    Catalog(String),
    /// Two catalog entries claimed the same app id.
    DuplicateAppId(String),
    /// A catalog entry named a repository that could not be parsed.
    CatalogRepo {
        /// The id of the entry that caused it.
        id: String,
        /// The repository value that could not be read.
        repo: String,
    },
    /// A colour was not a `#rrggbb` literal.
    BadColor {
        /// The id of the entry that caused it.
        id: String,
        /// The colour value that could not be read.
        value: String,
    },
    /// An asset extension in the catalog was not a usable archive extension.
    BadExtension(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRepo(value) => write!(
                f,
                "`{value}` is not a GitHub repository: expected `owner/name` or a github.com URL"
            ),
            Self::BadRepoPart { part, value } => {
                write!(f, "`{value}` is not a valid GitHub {part} name")
            }
            Self::ReleasePayload(reason) => write!(f, "the release list was not readable: {reason}"),
            Self::Catalog(reason) => write!(f, "the catalog could not be read: {reason}"),
            Self::DuplicateAppId(id) => write!(f, "two catalog entries share the id `{id}`"),
            Self::CatalogRepo { id, repo } => {
                write!(f, "catalog entry `{id}` names an unreadable repository `{repo}`")
            }
            Self::BadColor { id, value } => {
                write!(f, "catalog entry `{id}` has colour `{value}`, expected `#rrggbb`")
            }
            Self::BadExtension(ext) => write!(f, "`{ext}` is not a usable archive extension"),
        }
    }
}

impl std::error::Error for Error {}

/// The core's result type.
pub type Result<T> = core::result::Result<T, Error>;
