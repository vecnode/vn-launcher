//! vn-launcher's core.
//!
//! This crate is **pure**: no filesystem, no network, no clock, no environment. It
//! answers the questions that a launcher gets wrong at its peril — *which release
//! is newest*, *which attached file belongs on this machine*, *is what is installed
//! actually older*, *what should the catalog contain* — as ordinary functions over
//! ordinary data, so every one of those answers has a test next to it.
//!
//! The desktop side lives in `vnl-native` (IO) and the interface in `vnl-ui` (egui).
//! Nothing here knows that either exists.
//!
//! # The shape of a decision
//!
//! ```text
//!   Repo ──▶ feed ──▶ Release ──▶ choose() ──▶ AssetChoice
//!                                     │
//!   InstallState ─────────────────────┴──▶ compute() ──▶ AppStatus
//! ```
//!
//! [`catalog::Catalog`] supplies the [`catalog::AppEntry`] that names the repository
//! and the asset stems; [`asset::choose`] picks the file; [`status::compute`] turns
//! "what is installed" plus "what is offered" into the one verdict the interface
//! renders.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod asset;
pub mod catalog;
pub mod checksum;
pub mod color;
pub mod error;
pub mod manifest;
pub mod platform;
pub mod release;
pub mod repo;
pub mod status;
pub mod time;
pub mod version;

pub use asset::{Asset, AssetChoice};
pub use catalog::{AppEntry, AppKind, Catalog, InstallSpec, LaunchSpec};
pub use color::Color;
pub use error::{Error, Result};
pub use manifest::SCHEMA_VERSION;
pub use platform::{Arch, Os, Target};
pub use release::Release;
pub use repo::Repo;
pub use status::{AppStatus, Availability, InstallState, ReleaseInfo};
pub use version::Tag;

/// The version of vn-launcher itself, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The launcher's own repository.
///
/// Named here so the launcher can look itself up with exactly the machinery it uses
/// for every other app, rather than through a second code path that would drift.
pub const LAUNCHER_REPO: &str = "vecnode/vn-launcher";
