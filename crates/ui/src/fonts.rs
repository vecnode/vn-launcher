//! The embedded typefaces.
//!
//! A cross-platform application cannot rely on the system's UI font: Windows ships
//! Segoe UI, macOS ships SF Pro and Linux ships whatever is installed, and a window
//! that changes personality three ways is the fastest way to look unfinished. The
//! launcher therefore embeds its own faces, under the SIL Open Font License, and
//! keeps egui's bundled fonts behind them so glyphs Inter lacks — CJK, emoji —
//! still resolve instead of showing as boxes.
//!
//! Static faces are used rather than a variable font because the rasterizer behind
//! egui does not select variable axes reliably.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

use crate::theme::Text;

/// Inter Regular — body text.
const INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
/// Inter Medium — buttons and labels.
const INTER_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Inter-Medium.ttf");
/// Inter SemiBold — headings and titles.
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
/// Inter Bold — the strongest emphasis.
const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");
/// JetBrains Mono Regular — versions, paths and identifiers.
const JETBRAINS_MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");

/// A font family the interface names explicitly, because egui selects faces by
/// family rather than by weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Body text.
    Regular,
    /// Buttons and form labels.
    Medium,
    /// Headings and titles.
    SemiBold,
    /// The strongest emphasis, used sparingly.
    Bold,
}

impl Face {
    /// The egui family this face is registered under.
    #[must_use]
    pub fn family(self) -> FontFamily {
        match self {
            Self::Regular => FontFamily::Proportional,
            Self::Medium => FontFamily::Name(Arc::from("medium")),
            Self::SemiBold => FontFamily::Name(Arc::from("semibold")),
            Self::Bold => FontFamily::Name(Arc::from("bold")),
        }
    }

    /// A font at a given size in this face.
    #[must_use]
    pub fn at(self, size: f32) -> FontId {
        FontId::new(size, self.family())
    }
}

/// Register the launcher's faces on a context.
///
/// Called once at startup, and again whenever a window is recreated. Safe to call
/// more than once.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for (name, bytes) in [
        ("vnl-inter", INTER_REGULAR),
        ("vnl-inter-medium", INTER_MEDIUM),
        ("vnl-inter-semibold", INTER_SEMIBOLD),
        ("vnl-inter-bold", INTER_BOLD),
        ("vnl-mono", JETBRAINS_MONO),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    // Each family puts the launcher's face first and keeps egui's defaults after it,
    // so a glyph Inter does not carry — a CJK character, an emoji — still renders.
    prepend(&mut fonts, FontFamily::Proportional, "vnl-inter");
    prepend(&mut fonts, FontFamily::Monospace, "vnl-mono");
    prepend(&mut fonts, Face::Medium.family(), "vnl-inter-medium");
    prepend(&mut fonts, Face::SemiBold.family(), "vnl-inter-semibold");
    prepend(&mut fonts, Face::Bold.family(), "vnl-inter-bold");

    // The named families fall back to the proportional stack, then to the default
    // faces, so a missing weight degrades instead of vanishing.
    let proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for face in [Face::Medium, Face::SemiBold, Face::Bold] {
        let family = face.family();
        let entry = fonts.families.entry(family).or_default();
        for name in &proportional {
            if !entry.contains(name) {
                entry.push(name.clone());
            }
        }
    }

    ctx.set_fonts(fonts);
}

/// Put a font at the front of a family, if it is not already there.
fn prepend(fonts: &mut FontDefinitions, family: FontFamily, name: &str) {
    let entry = fonts.families.entry(family).or_default();
    entry.retain(|existing| existing != name);
    entry.insert(0, name.to_owned());
}

/// The type scale the interface uses, resolved against the registered faces.
#[must_use]
pub fn scale() -> Text {
    Text {
        display: Face::SemiBold.at(25.0),
        title: Face::SemiBold.at(19.0),
        heading: Face::SemiBold.at(15.5),
        body: Face::Regular.at(14.0),
        label: Face::Medium.at(13.5),
        small: Face::Regular.at(12.5),
        micro: Face::Medium.at(11.5),
        mono: FontId::new(12.5, FontFamily::Monospace),
    }
}
