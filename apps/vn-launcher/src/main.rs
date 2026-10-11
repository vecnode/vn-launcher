//! vn-launcher — install, update and open the vecnode applications.
//!
//! The binary is the only part of the workspace that opens a window. Everything it
//! needs already exists as a library:
//!
//! - [`vnl_core`] decides *what is true* — which release is newest, which file fits
//!   this machine, whether what is installed is older;
//! - [`vnl_native`] does the reaching out — HTTP, the feed cache, finding an app on
//!   disk, starting a process;
//! - [`vnl_ui`] draws, and returns the things a person asked for.
//!
//! This crate joins them: it owns the world, runs the background worker that keeps
//! it current, and carries out what the interface asks for.
//!
//! ```text
//!   worker thread ──▶ Response ──▶ Shell.world ──▶ vnl_ui::Ui::show
//!         ▲                                             │
//!         └────────────── Request ◀──── Action ◀─────────┘
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]
// A binary has no public API — every item in it is internal to the program — so the
// `unreachable_pub` lint that guards the three library crates has nothing to say
// here. The libraries keep it.
#![allow(unreachable_pub)]
// A release build is a GUI application and must not open a console window. The
// debug build keeps it, because that console is where the log goes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod shell;
mod worker;

/// The window's starting size.
const STARTING_SIZE: [f32; 2] = [1180.0, 780.0];

/// The smallest the window may be made before the layout stops reflowing.
const MINIMUM_SIZE: [f32; 2] = [900.0, 600.0];

/// The application's own mark, as unmultiplied RGBA.
///
/// Embedded as raw pixels rather than as a PNG so that putting an icon on the window
/// and the taskbar costs no image decoder — `egui::IconData` takes exactly these three
/// fields, and the file comes from the same geometry `ui/src/icons.rs` draws with.
///
/// It is the mark inverted onto the application's black rather than the accent blue the
/// sidebar uses. An icon has to hold its own against whatever is behind it: a bare
/// black glyph disappears on a dark taskbar and a bare white one on a light taskbar,
/// whereas a black tile with a white glyph reads on both.
const LOGO_RGBA: &[u8] = include_bytes!("../../../assets/logo/vn-launcher-app.rgba");

/// The logo is square, and this many pixels on a side.
const LOGO_SIZE: u32 = 64;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("vn-launcher")
            .with_app_id("vn-launcher")
            .with_inner_size(STARTING_SIZE)
            .with_min_inner_size(MINIMUM_SIZE)
            .with_icon(egui::IconData {
                rgba: LOGO_RGBA.to_vec(),
                width: LOGO_SIZE,
                height: LOGO_SIZE,
            }),
        ..Default::default()
    };

    eframe::run_native(
        "vn-launcher",
        options,
        Box::new(|cc| Ok(Box::new(shell::Shell::new(cc)))),
    )
}
