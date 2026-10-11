//! vn-launcher's desktop backend.
//!
//! This is the only crate that touches the filesystem, the network or other
//! processes. Everything here is in service of three questions the interface asks:
//!
//! - *what does GitHub offer for this app?* — [`feed::Feed`], reading through
//!   [`cache::FeedCache`] so the launcher stays a polite guest of the public API;
//! - *is it already on this machine?* — [`detect`];
//! - *can I start it or open its page?* — [`launch`].
//!
//! There is no account, no token and no login. Every request goes to the public
//! GitHub API anonymously, and the launcher is designed to work that way.
//!
//! Two things this crate deliberately does **not** do yet: download, verify and
//! unpack a release, and write installation records. Those belong together with the
//! release conventions described in `docs/RELEASES.md`, and arrive once those are
//! settled.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod cache;
pub mod detect;
pub mod error;
pub mod feed;
pub mod http;
pub mod launch;
pub mod paths;
pub mod user_catalog;

pub use cache::FeedCache;
pub use detect::{Detection, Found};
pub use error::{Error, Result};
pub use feed::{Feed, Probe};
pub use http::Client;
pub use paths::Paths;
