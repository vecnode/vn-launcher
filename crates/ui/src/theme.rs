//! The launcher's look: colour tokens, spacing, radii and type.
//!
//! Everything visual is named here rather than written at the call site. That is
//! what keeps a status badge in the library view and the same badge in the detail
//! view from drifting apart, and it is what makes adding a second theme a matter of
//! supplying another palette rather than auditing every widget.

use egui::{Color32, CornerRadius, FontId, Margin, Stroke, Vec2, vec2};

use crate::fonts;

/// The 4-pixel spacing scale. Every margin and gap is one of these.
pub const SPACE_XS: f32 = 4.0;
/// Small gap.
pub const SPACE_SM: f32 = 8.0;
/// Medium gap.
pub const SPACE_MD: f32 = 12.0;
/// Large gap.
pub const SPACE_LG: f32 = 16.0;
/// Section gap.
pub const SPACE_XL: f32 = 24.0;
/// Page gutter.
pub const SPACE_2XL: f32 = 32.0;

/// Corner radius for small controls.
pub const RADIUS_SM: u8 = 6;
/// Corner radius for cards and inputs.
pub const RADIUS_MD: u8 = 10;
/// Corner radius for large surfaces.
pub const RADIUS_LG: u8 = 14;

/// Width of the navigation rail.
pub const SIDEBAR_WIDTH: f32 = 232.0;
/// Height of the title bar.
pub const TOPBAR_HEIGHT: f32 = 56.0;
/// Narrowest a library card may become before the grid drops a column.
pub const CARD_MIN_WIDTH: f32 = 300.0;
/// Height of a library card.
///
/// Sized so that a name, a tagline, a status row, an explanation and the action row all
/// fit with room to spare at the bottom. It was 184, which left the action row one
/// padding step off the card's own border whenever anything above it ran to a second
/// line — close enough to read as touching it.
pub const CARD_HEIGHT: f32 = 200.0;
/// Height reserved for a card's or row's action buttons.
pub const ACTION_ROW_HEIGHT: f32 = 32.0;

/// Which theme the person asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThemeChoice {
    /// Follow the operating system.
    #[default]
    System,
    /// Always dark.
    Dark,
    /// Always light.
    Light,
}

impl ThemeChoice {
    /// Every choice, in menu order.
    pub const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

    /// How it is written in the interface and in the settings file.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// Every colour the interface uses, named by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The window background.
    pub bg_base: Color32,
    /// A recessed surface, such as the navigation rail.
    pub bg_panel: Color32,
    /// A raised surface, such as a card.
    pub bg_elevated: Color32,
    /// A raised surface under the pointer.
    pub bg_hover: Color32,
    /// A raised surface being pressed.
    pub bg_active: Color32,
    /// Text inputs and other wells.
    pub bg_input: Color32,
    /// Hairline separators.
    pub border_subtle: Color32,
    /// Borders that need to be seen.
    pub border_strong: Color32,
    /// Body text.
    pub text_primary: Color32,
    /// Supporting text.
    pub text_secondary: Color32,
    /// Hints and metadata.
    pub text_tertiary: Color32,
    /// The launcher's own accent.
    pub accent: Color32,
    /// The accent under the pointer.
    pub accent_hover: Color32,
    /// The accent being pressed.
    pub accent_pressed: Color32,
    /// Text drawn on the accent.
    pub on_accent: Color32,
    /// Something is well.
    pub success: Color32,
    /// Something needs attention soon.
    pub warning: Color32,
    /// Something is wrong.
    pub danger: Color32,
    /// Neutral information.
    pub info: Color32,
}

impl Palette {
    /// The dark palette.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            bg_base: Color32::from_rgb(0x0b, 0x0d, 0x10),
            bg_panel: Color32::from_rgb(0x11, 0x14, 0x18),
            bg_elevated: Color32::from_rgb(0x16, 0x1a, 0x20),
            bg_hover: Color32::from_rgb(0x1d, 0x22, 0x2a),
            bg_active: Color32::from_rgb(0x25, 0x2c, 0x36),
            bg_input: Color32::from_rgb(0x0e, 0x11, 0x15),
            border_subtle: Color32::from_rgb(0x22, 0x28, 0x31),
            border_strong: Color32::from_rgb(0x33, 0x3c, 0x48),
            text_primary: Color32::from_rgb(0xe9, 0xec, 0xf0),
            text_secondary: Color32::from_rgb(0x9d, 0xa7, 0xb4),
            text_tertiary: Color32::from_rgb(0x6c, 0x77, 0x85),
            accent: Color32::from_rgb(0x4c, 0x8d, 0xff),
            accent_hover: Color32::from_rgb(0x6b, 0xa0, 0xff),
            accent_pressed: Color32::from_rgb(0x3a, 0x7a, 0xe8),
            on_accent: Color32::from_rgb(0xff, 0xff, 0xff),
            success: Color32::from_rgb(0x46, 0xc4, 0x60),
            warning: Color32::from_rgb(0xdb, 0xa1, 0x2c),
            danger: Color32::from_rgb(0xf8, 0x5c, 0x54),
            info: Color32::from_rgb(0x60, 0xa8, 0xfa),
        }
    }

    /// The light palette.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            bg_base: Color32::from_rgb(0xff, 0xff, 0xff),
            bg_panel: Color32::from_rgb(0xf6, 0xf7, 0xf9),
            bg_elevated: Color32::from_rgb(0xff, 0xff, 0xff),
            bg_hover: Color32::from_rgb(0xef, 0xf1, 0xf4),
            bg_active: Color32::from_rgb(0xe4, 0xe8, 0xed),
            bg_input: Color32::from_rgb(0xf6, 0xf7, 0xf9),
            border_subtle: Color32::from_rgb(0xe4, 0xe7, 0xeb),
            border_strong: Color32::from_rgb(0xc7, 0xcd, 0xd5),
            text_primary: Color32::from_rgb(0x14, 0x17, 0x1a),
            text_secondary: Color32::from_rgb(0x59, 0x62, 0x6d),
            text_tertiary: Color32::from_rgb(0x87, 0x8f, 0x99),
            accent: Color32::from_rgb(0x25, 0x63, 0xeb),
            accent_hover: Color32::from_rgb(0x3b, 0x76, 0xf0),
            accent_pressed: Color32::from_rgb(0x1d, 0x4e, 0xc4),
            on_accent: Color32::from_rgb(0xff, 0xff, 0xff),
            success: Color32::from_rgb(0x1a, 0x7f, 0x37),
            warning: Color32::from_rgb(0x9a, 0x67, 0x00),
            danger: Color32::from_rgb(0xc4, 0x25, 0x1c),
            info: Color32::from_rgb(0x1d, 0x4e, 0xc4),
        }
    }

    /// A translucent version of a colour, for badge and tile backgrounds.
    #[must_use]
    pub fn tint(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }
}

/// The type scale, resolved for the registered faces.
///
/// Not `Copy`: `FontId` carries a `FontFamily`, which can name a family through an
/// `Arc<str>`.
#[derive(Debug, Clone)]
pub struct Text {
    /// The largest text, used once per screen at most.
    pub display: FontId,
    /// A page or dialog title.
    pub title: FontId,
    /// A section heading.
    pub heading: FontId,
    /// Body text.
    pub body: FontId,
    /// A control's label.
    pub label: FontId,
    /// Supporting text.
    pub small: FontId,
    /// Hints and overlines.
    pub micro: FontId,
    /// Versions, paths and identifiers.
    pub mono: FontId,
}

/// The theme in force.
#[derive(Debug, Clone)]
pub struct Theme {
    /// The colours.
    pub palette: Palette,
    /// The type scale.
    pub text: Text,
    /// Whether this is the dark theme.
    pub dark: bool,
}

impl Theme {
    /// The dark theme.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            palette: Palette::dark(),
            text: fonts::scale(),
            dark: true,
        }
    }

    /// The light theme.
    #[must_use]
    pub fn light() -> Self {
        Self {
            palette: Palette::light(),
            text: fonts::scale(),
            dark: false,
        }
    }

    /// Resolve a choice against what the operating system prefers.
    #[must_use]
    pub fn resolve(choice: ThemeChoice, system_prefers_dark: bool) -> Self {
        let dark = match choice {
            ThemeChoice::System => system_prefers_dark,
            ThemeChoice::Dark => true,
            ThemeChoice::Light => false,
        };
        if dark { Self::dark() } else { Self::light() }
    }

    /// Register the launcher's faces and install this theme's style.
    ///
    /// Safe to call at any time, but prefer [`Self::apply`] once the fonts are in:
    /// `egui::Context::set_fonts` takes effect at the start of the *next* pass, so
    /// text laid out with a named family during the same pass that registered it
    /// panics. The shell therefore registers the fonts before the first frame.
    pub fn install(&self, ctx: &egui::Context) {
        fonts::install(ctx);
        self.apply(ctx);
    }

    /// Install this theme's style, leaving the registered fonts alone.
    ///
    /// egui keeps a style per theme variant, so both are written: the launcher decides
    /// every colour itself and does not want egui's light or dark defaults leaking
    /// through in either case.
    pub fn apply(&self, ctx: &egui::Context) {
        let palette = self.palette;
        let text = self.text.clone();
        let dark = self.dark;

        ctx.all_styles_mut(move |style| {
            style.visuals = visuals(dark, palette);
            style.text_styles = [
                (egui::TextStyle::Heading, text.title.clone()),
                (egui::TextStyle::Body, text.body.clone()),
                (egui::TextStyle::Monospace, text.mono.clone()),
                (egui::TextStyle::Button, text.label.clone()),
                (egui::TextStyle::Small, text.small.clone()),
            ]
            .into();
            style.spacing.item_spacing = vec2(SPACE_SM, SPACE_SM);
            style.spacing.button_padding = vec2(SPACE_MD, 7.0);
            style.spacing.menu_margin = Margin::same(6);
            style.spacing.window_margin = Margin::same(SPACE_MD as i8);
            style.spacing.interact_size = Vec2::new(0.0, 30.0);
            style.spacing.scroll.bar_width = 10.0;
            style.spacing.scroll.floating = true;
        });
    }
}

/// Map the palette onto egui's own widget styling.
fn visuals(dark: bool, palette: Palette) -> egui::Visuals {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = palette.bg_base;
    visuals.window_fill = palette.bg_elevated;
    visuals.extreme_bg_color = palette.bg_input;
    visuals.faint_bg_color = palette.bg_panel;
    visuals.override_text_color = Some(palette.text_primary);
    visuals.hyperlink_color = palette.accent;
    visuals.window_stroke = Stroke::new(1.0, palette.border_subtle);
    visuals.window_corner_radius = CornerRadius::same(RADIUS_MD);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_MD);
    visuals.selection.bg_fill = Palette::tint(palette.accent, 70);
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = palette.bg_panel;
    widgets.noninteractive.weak_bg_fill = palette.bg_panel;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border_subtle);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_secondary);
    widgets.noninteractive.corner_radius = CornerRadius::same(RADIUS_MD);

    widgets.inactive.bg_fill = palette.bg_elevated;
    widgets.inactive.weak_bg_fill = palette.bg_elevated;
    widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border_subtle);
    widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    widgets.inactive.corner_radius = CornerRadius::same(RADIUS_MD);

    widgets.hovered.bg_fill = palette.bg_hover;
    widgets.hovered.weak_bg_fill = palette.bg_hover;
    widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    widgets.hovered.corner_radius = CornerRadius::same(RADIUS_MD);

    widgets.active.bg_fill = palette.bg_active;
    widgets.active.weak_bg_fill = palette.bg_active;
    widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    widgets.active.fg_stroke = Stroke::new(1.0, palette.text_primary);
    widgets.active.corner_radius = CornerRadius::same(RADIUS_MD);

    widgets.open.bg_fill = palette.bg_hover;
    widgets.open.weak_bg_fill = palette.bg_hover;
    widgets.open.bg_stroke = Stroke::new(1.0, palette.border_strong);
    widgets.open.fg_stroke = Stroke::new(1.0, palette.text_primary);
    widgets.open.corner_radius = CornerRadius::same(RADIUS_MD);

    visuals
}
