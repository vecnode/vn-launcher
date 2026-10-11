//! vn-launcher's interface.
//!
//! An egui front end with no knowledge of the filesystem, the network or other
//! processes. It draws a [`World`] and returns a list of [`Action`]s; the desktop
//! shell in `apps/vn-launcher` carries them out. That boundary is what lets the
//! whole interface be exercised against a hand-written [`World`], with no server and
//! no fixtures.
//!
//! # Layout
//!
//! ```text
//!   ┌──────────┬───────────────────────────────────────┐
//!   │ mark     │  page title        search   [check]   │
//!   ├──────────┼───────────────────────────────────────┤
//!   │ Library  │                                       │
//!   │ Updates  │   the current view                    │
//!   │ Add      │                                       │
//!   │ Settings │                                       │
//!   │ ──────── │                                       │
//!   │ version  │                                       │
//!   └──────────┴───────────────────────────────────────┘
//! ```
//!
//! The rail is added before the bar so that it runs the full height of the window.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod app;
pub mod fonts;
pub mod icons;
pub mod model;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::Ui;
pub use model::{Action, AppCard, LauncherUpdate, Notice, ProbeReport, Tab, Tone, World};
pub use theme::{Theme, ThemeChoice};
pub use vnl_core::LAUNCHER_REPO;

/// Where the release convention every repository must follow is written down.
///
/// The interface links to it wherever it has to explain why installing is not
/// offered yet, so the explanation is never a dead end.
pub const RELEASE_CONVENTION_URL: &str = "https://github.com/vecnode/vn-launcher/blob/main/docs/RELEASES.md";
