//! Errors from the desktop side.
//!
//! Every variant carries enough context to be shown to a person without a
//! backtrace: what was being attempted, and what the operating system or the
//! network said.

use core::fmt;

/// What went wrong in the desktop backend.
#[derive(Debug)]
pub enum Error {
    /// The pure core refused something.
    Core(vnl_core::Error),
    /// The filesystem refused something.
    Io {
        /// What was being attempted.
        context: String,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// The request never completed.
    Network {
        /// The URL.
        url: String,
        /// Why.
        reason: String,
    },
    /// The request completed with a status the launcher does not handle.
    Http {
        /// The URL.
        url: String,
        /// The status code.
        status: u16,
        /// GitHub's own explanation, when it gave one.
        message: String,
    },
    /// The response was larger than the launcher is willing to read.
    TooLarge {
        /// The URL.
        url: String,
        /// The cap in bytes.
        limit: u64,
    },
    /// The on-disk cache could not be read or written.
    Cache(String),
    /// A process could not be started.
    Spawn {
        /// The program.
        program: String,
        /// What the operating system said.
        reason: String,
    },
    /// No usable platform directory was reported by the operating system.
    NoConfigDir,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(inner) => write!(f, "{inner}"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Network { url, reason } => write!(f, "could not reach {url}: {reason}"),
            Self::Http { url, status, message } => {
                if message.is_empty() {
                    write!(f, "{url} returned HTTP {status}")
                } else {
                    write!(f, "{url} returned HTTP {status}: {message}")
                }
            }
            Self::TooLarge { url, limit } => {
                write!(f, "{url} sent more than the {limit} bytes the launcher will read")
            }
            Self::Cache(reason) => write!(f, "the local cache could not be used: {reason}"),
            Self::Spawn { program, reason } => write!(f, "could not start `{program}`: {reason}"),
            Self::NoConfigDir => {
                f.write_str("the operating system did not report a configuration directory for this user")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(inner) => Some(inner),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<vnl_core::Error> for Error {
    fn from(value: vnl_core::Error) -> Self {
        Self::Core(value)
    }
}

/// The desktop side's result type.
pub type Result<T> = core::result::Result<T, Error>;

/// Attach a description of what was being attempted to an IO failure.
pub(crate) fn io_context(context: impl Into<String>, source: std::io::Error) -> Error {
    Error::Io {
        context: context.into(),
        source,
    }
}
