//! vn-launcher's desktop backend.
//!
//! This is the only crate that touches the filesystem, the network or other
//! processes. Everything here is in service of four questions the interface asks:
//!
//! - *what does GitHub offer for this app?* — [`feed::Feed`], reading through
//!   [`cache::FeedCache`] so the launcher stays a polite guest of the public API;
//! - *is it already on this machine?* — [`detect`];
//! - *can I start it or open its page?* — [`launch`];
//! - *can I take it back out again?* — [`uninstall`], which only ever deletes inside
//!   the launcher's own install root, and [`user_catalog::remove`], which writes the
//!   tombstone that keeps a removed app out of the catalog.
//!
//! There is no account, no token and no login. Every request goes to the public
//! GitHub API anonymously, and the launcher is designed to work that way.
//!
//! Downloading, verifying, unpacking and recording an installation are deliberately
//! **not** here yet: they arrive together, and the plan for them is `docs/INSTALL.md`.
//! Removal does not wait for them, because removing something the launcher never put
//! there is not possible in the first place — [`uninstall`] has nothing to delete
//! until the other half exists.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod cache;
pub mod detect;
pub mod error;
pub mod feed;
pub mod http;
pub mod launch;
pub mod paths;
pub mod uninstall;
pub mod user_catalog;

pub use cache::FeedCache;
pub use detect::{Detection, Found};
pub use error::{Error, Result};
pub use feed::{Feed, Probe};
pub use http::Client;
pub use paths::Paths;
pub use uninstall::Removal;
pub use user_catalog::Overlay;
