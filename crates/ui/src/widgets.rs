//! The interface's building blocks.
//!
//! egui's stock widgets are functional and generic; a launcher that looks finished
//! needs a smaller vocabulary used consistently. Everything here takes the
//! [`Theme`] rather than reading colours from the ambient style, so a widget looks
//! the same wherever it is placed and a second theme is a palette swap.

use egui::{
    Color32, CornerRadius, FontId, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};

use crate::fonts::Face;
use crate::icons::{self, Icon};
use crate::model::{Notice, Tone};
use crate::theme::{RADIUS_MD, RADIUS_SM, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS, Theme};

/// How prominent a button is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// The one thing to do on a surface.
    Primary,
    /// Everything else.
    Secondary,
    /// A low-emphasis action.
    Ghost,
    /// Something destructive.
    Danger,
}

/// The padding a card leaves between its edge and its contents.
pub const CARD_PADDING: f32 = SPACE_LG;

/// The surface a card is drawn on.
pub fn card_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(theme.palette.bg_elevated)
        .stroke(Stroke::new(1.0, theme.palette.border_subtle))
        .corner_radius(CornerRadius::same(RADIUS_MD))
        .inner_margin(Margin::same(CARD_PADDING as i8))
}

/// Draw a card filling the width available to it, and run `add_contents` inside.
///
/// The width handed to the contents is the available width *minus the card's own
/// padding*. Telling the contents to be the full available width instead makes the
/// card wider than the space it was given — which is what once squeezed the detail
/// page's two-column grid into a sliver a few characters wide, because egui's `Grid`
/// divides whatever width it is handed rather than asking for more.
pub fn card<R>(ui: &mut Ui, theme: &Theme, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let width = (ui.available_width() - 2.0 * CARD_PADDING).max(160.0);
    card_frame(theme)
        .show(ui, |ui| {
            ui.set_width(width);
            add_contents(ui)
        })
        .inner
}

/// The foreground and background for a tone.
#[must_use]
pub fn tone_colors(theme: &Theme, tone: Tone) -> (Color32, Color32) {
    let palette = &theme.palette;
    let alpha = if theme.dark { 38 } else { 26 };
    let (foreground, base) = match tone {
        Tone::Neutral => (palette.text_secondary, palette.border_strong),
        Tone::Accent => (palette.accent, palette.accent),
        Tone::Success => (palette.success, palette.success),
        Tone::Warning => (palette.warning, palette.warning),
        Tone::Danger => (palette.danger, palette.danger),
        Tone::Info => (palette.info, palette.info),
    };
    (
        foreground,
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha),
    )
}

/// A rounded pill of small upper-case text: a status.
pub fn badge(ui: &mut Ui, theme: &Theme, text: &str, tone: Tone) -> Response {
    let (foreground, background) = tone_colors(theme, tone);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_uppercase(), theme.text.micro.clone(), foreground);
    let size = vec2(galley.size().x + SPACE_SM * 2.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, RADIUS_SM, background);
        painter.galley(
            pos2(rect.left() + SPACE_SM, rect.center().y - galley.size().y / 2.0),
            galley,
            foreground,
        );
    }
    response
}

/// A monogram tile in an app's brand colour.
///
/// The launcher has no icon per repository, so the mark is drawn: the brand colour
/// as the surface and the app's initials on it, in whichever of black or white the
/// contrast calculation prefers.
pub fn brand_tile(ui: &mut Ui, size: f32, colour: Color32, monogram: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let ink = readable_ink(colour);
    let font = Face::SemiBold.at(size * 0.38);
    let galley = ui.painter().layout_no_wrap(monogram.to_owned(), font, ink);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS_MD), colour);
    painter.galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        ink,
    );
    response
}

/// Black or white, whichever reads better on `background`.
#[must_use]
pub fn readable_ink(background: Color32) -> Color32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(background.r()) + 0.7152 * linear(background.g()) + 0.0722 * linear(background.b());
    let against_white = 1.05 / (luminance + 0.05);
    let against_black = (luminance + 0.05) / 0.05;
    if against_black >= against_white {
        Color32::from_rgb(0x0b, 0x0d, 0x10)
    } else {
        Color32::from_rgb(0xff, 0xff, 0xff)
    }
}

/// A button, drawn to the launcher's own spec.
pub fn button(
    ui: &mut Ui,
    theme: &Theme,
    kind: ButtonKind,
    icon: Option<Icon>,
    text: &str,
    enabled: bool,
) -> Response {
    button_impl(ui, theme, kind, icon, text, enabled, None)
}

/// The same button, stretched across the width available.
///
/// The rail's About sits directly above the navigation, and a button that hugged its
/// label there would read as a different kind of thing from the items beside it.
pub fn button_full(
    ui: &mut Ui,
    theme: &Theme,
    kind: ButtonKind,
    icon: Option<Icon>,
    text: &str,
    enabled: bool,
) -> Response {
    let width = ui.available_width();
    button_impl(ui, theme, kind, icon, text, enabled, Some(width))
}

/// The one implementation behind [`button`] and [`button_full`].
fn button_impl(
    ui: &mut Ui,
    theme: &Theme,
    kind: ButtonKind,
    icon: Option<Icon>,
    text: &str,
    enabled: bool,
    width_override: Option<f32>,
) -> Response {
    let palette = &theme.palette;
    let (base, hover, pressed, foreground, stroke) = match kind {
        ButtonKind::Primary => (
            palette.accent,
            palette.accent_hover,
            palette.accent_pressed,
            palette.on_accent,
            Stroke::NONE,
        ),
        ButtonKind::Secondary => (
            palette.bg_elevated,
            palette.bg_hover,
            palette.bg_active,
            palette.text_primary,
            Stroke::new(1.0, palette.border_subtle),
        ),
        ButtonKind::Ghost => (
            Color32::TRANSPARENT,
            palette.bg_hover,
            palette.bg_active,
            palette.text_secondary,
            Stroke::NONE,
        ),
        ButtonKind::Danger => (
            palette.bg_elevated,
            crate::theme::Palette::tint(palette.danger, 46),
            crate::theme::Palette::tint(palette.danger, 74),
            palette.danger,
            Stroke::new(1.0, crate::theme::Palette::tint(palette.danger, 90)),
        ),
    };

    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), theme.text.label.clone(), foreground);

    let icon_size = 15.0;
    let gap = if icon.is_some() { SPACE_SM } else { 0.0 };
    let width = galley.size().x + if icon.is_some() { icon_size + gap } else { 0.0 } + SPACE_MD * 2.0;
    let size = vec2(width_override.unwrap_or(width.max(64.0)), 32.0);

    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);

    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            base.gamma_multiply(0.5)
        } else if response.is_pointer_button_down_on() {
            pressed
        } else if response.hovered() {
            hover
        } else {
            base
        };
        let foreground = if enabled {
            foreground
        } else {
            palette.text_tertiary
        };

        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(RADIUS_MD),
            fill,
            stroke,
            StrokeKind::Inside,
        );

        let mut left = rect.left() + SPACE_MD;
        if let Some(icon) = icon {
            let icon_rect = Rect::from_min_size(
                pos2(left, rect.center().y - icon_size / 2.0),
                vec2(icon_size, icon_size),
            );
            icons::paint(painter, icon, icon_rect, foreground, 1.6);
            left += icon_size + gap;
        }
        painter.galley(
            pos2(left, rect.center().y - galley.size().y / 2.0),
            galley,
            foreground,
        );
    }

    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// A square button holding only an icon.
pub fn icon_button(ui: &mut Ui, theme: &Theme, icon: Icon, tooltip: &str, enabled: bool) -> Response {
    let size = 32.0;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), sense);

    if ui.is_rect_visible(rect) {
        let palette = &theme.palette;
        if enabled && (response.hovered() || response.is_pointer_button_down_on()) {
            let fill = if response.is_pointer_button_down_on() {
                palette.bg_active
            } else {
                palette.bg_hover
            };
            ui.painter()
                .rect_filled(rect, CornerRadius::same(RADIUS_MD), fill);
        }
        let colour = if enabled {
            if response.hovered() {
                palette.text_primary
            } else {
                palette.text_secondary
            }
        } else {
            palette.text_tertiary
        };
        let inner = rect.shrink(8.0);
        icons::paint(ui.painter(), icon, inner, colour, 1.6);
    }

    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

/// A square icon button that shows whether it is the chosen one.
///
/// [`icon_button`] cannot say "this is the current one": a hover and a selection would
/// look the same. The chosen one therefore gets the rail's own treatment — an accent
/// tint behind it and accent ink on it — which is how a person already reads the
/// selected tab in the rail.
pub fn icon_toggle(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    selected: bool,
    tooltip: &str,
    enabled: bool,
) -> Response {
    let size = 32.0;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), sense);

    if ui.is_rect_visible(rect) {
        let palette = &theme.palette;
        let fill = if selected {
            crate::theme::Palette::tint(palette.accent, if theme.dark { 46 } else { 30 })
        } else if enabled && response.is_pointer_button_down_on() {
            palette.bg_active
        } else if enabled && response.hovered() {
            palette.bg_hover
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(RADIUS_MD), fill);
        }

        let colour = if selected {
            palette.accent
        } else if !enabled {
            palette.text_tertiary
        } else if response.hovered() {
            palette.text_primary
        } else {
            palette.text_secondary
        };
        icons::paint(ui.painter(), icon, rect.shrink(8.0), colour, 1.6);
    }

    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

/// A small upper-case label above a group.
pub fn overline(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(theme.text.micro.clone())
            .color(theme.palette.text_tertiary),
    );
}

/// A section heading with an optional explanation.
pub fn section(ui: &mut Ui, theme: &Theme, title: &str, detail: Option<&str>) {
    ui.add_space(SPACE_XS);
    ui.label(
        RichText::new(title)
            .font(theme.text.heading.clone())
            .color(theme.palette.text_primary),
    );
    if let Some(detail) = detail {
        ui.add_space(SPACE_XS);
        ui.label(
            RichText::new(detail)
                .font(theme.text.small.clone())
                .color(theme.palette.text_secondary),
        );
    }
    ui.add_space(SPACE_SM);
}

/// A label and its value, stacked.
pub fn field(ui: &mut Ui, theme: &Theme, label: &str, value: &str, monospace: bool) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label.to_uppercase())
                .font(theme.text.micro.clone())
                .color(theme.palette.text_tertiary),
        );
        ui.add_space(2.0);
        let text = RichText::new(value).color(theme.palette.text_primary);
        ui.label(if monospace {
            text.font(theme.text.mono.clone())
        } else {
            text.font(theme.text.body.clone())
        });
    });
}

/// A hairline rule.
pub fn separator(ui: &mut Ui, theme: &Theme) {
    ui.add_space(SPACE_MD);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, theme.palette.border_subtle);
    ui.add_space(SPACE_MD);
}

/// A centred explanation for an empty surface.
///
/// Returns the primary action's response, when one was offered and clicked.
pub fn empty_state(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    title: &str,
    detail: &str,
    action: Option<(&str, Icon)>,
) -> Option<Response> {
    let mut clicked = None;
    ui.vertical_centered(|ui| {
        ui.add_space(SPACE_XL * 2.0);
        let (rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
        icons::paint(ui.painter(), icon, rect, theme.palette.text_tertiary, 1.4);
        ui.add_space(SPACE_LG);
        ui.label(
            RichText::new(title)
                .font(theme.text.title.clone())
                .color(theme.palette.text_primary),
        );
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(detail)
                .font(theme.text.small.clone())
                .color(theme.palette.text_secondary),
        );
        if let Some((label, icon)) = action {
            ui.add_space(SPACE_XL);
            ui.vertical_centered(|ui| {
                ui.set_max_width(220.0);
                let response = button(ui, theme, ButtonKind::Primary, Some(icon), label, true);
                if response.clicked() {
                    clicked = Some(response);
                }
            });
        }
        ui.add_space(SPACE_XL);
    });
    clicked
}

/// A coloured banner for a message.
///
/// The returned response is the dismiss button, so the caller learns about a
/// dismissal and nothing else.
pub fn notice_banner(ui: &mut Ui, theme: &Theme, notice: &Notice) -> Response {
    let (foreground, background) = tone_colors(theme, notice.tone);
    let frame = egui::Frame::new()
        .fill(background)
        .stroke(Stroke::new(1.0, crate::theme::Palette::tint(foreground, 90)))
        .corner_radius(CornerRadius::same(RADIUS_MD))
        .inner_margin(Margin::symmetric(SPACE_MD as i8, SPACE_SM as i8));

    frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                icons::paint(ui.painter(), Icon::Alert, rect, foreground, 1.6);
                ui.add_space(SPACE_SM);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dismiss = icon_button(ui, theme, Icon::Close, "Dismiss", true);
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label(
                            RichText::new(&notice.title)
                                .font(theme.text.label.clone())
                                .color(theme.palette.text_primary),
                        );
                        if let Some(detail) = &notice.detail {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(detail)
                                    .font(theme.text.small.clone())
                                    .color(theme.palette.text_secondary),
                            );
                        }
                    });
                    dismiss
                })
                .inner
            })
            .inner
        })
        .inner
}

/// A label with an icon before it, on one line.
pub fn icon_label(ui: &mut Ui, theme: &Theme, icon: Icon, text: &str, tone: Tone) {
    let (foreground, _) = tone_colors(theme, tone);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
        icons::paint(ui.painter(), icon, rect, foreground, 1.5);
        ui.add_space(SPACE_XS);
        ui.label(
            RichText::new(text)
                .font(theme.text.small.clone())
                .color(theme.palette.text_secondary),
        );
    });
}

/// A heading written in a specific font and colour.
#[must_use]
pub fn text(font: FontId, colour: Color32, value: impl Into<String>) -> RichText {
    RichText::new(value).font(font).color(colour)
}
