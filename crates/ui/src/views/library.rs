//! The library: every tracked app, as a grid of cards or a list of rows.
//!
//! The grid reflows with the window rather than scrolling sideways, and the whole
//! card is a target for opening the detail page — with the card's own buttons
//! taking precedence, because a launcher whose Launch button opens a page instead
//! would be maddening.
//!
//! The list is the same information at a different density. A card has room for a
//! tagline and a sentence; a row has room for four apps where one card would have
//! gone, which is what a library of ten repositories needs. The two views share the
//! same actions and the same buttons, so nothing is possible in one and not the other.

use egui::{CornerRadius, RichText, Sense, Stroke, StrokeKind, vec2};

use crate::app::Ui as State;
use crate::icons::Icon;
use crate::model::{Action, AppCard, LibraryView, Tab, World, describe, explain};
use crate::theme::{
    ACTION_ROW_HEIGHT, CARD_HEIGHT, CARD_MIN_WIDTH, RADIUS_LG, RADIUS_MD, SPACE_LG, SPACE_MD, SPACE_SM,
    SPACE_XS, Theme,
};
use crate::views::{self};
use crate::widgets::{self};

/// The height of one row in the list view.
///
/// Tall enough for a 32-point tile and two lines of text with the card's own padding
/// top and bottom, and no taller: the point of the list is how many apps fit on a
/// screen, and every point above this one costs a row somewhere down the page.
const ROW_HEIGHT: f32 = 72.0;

/// The width the leading action of a row is given, before a narrow window shrinks it.
const ACTION_COLUMN_WIDTH: f32 = 200.0;

/// Draw the library.
pub fn show(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    if world.apps.is_empty() {
        if widgets::empty_state(
            ui,
            theme,
            Icon::Package,
            "No apps are configured",
            "The catalog is empty. Add a repository to get started.",
            Some(("Add a repository", Icon::Plus)),
        )
        .is_some()
        {
            actions.push(Action::SetTab(Tab::Discover));
        }
        return;
    }

    let cards = views::filtered(world, &state.search);
    if cards.is_empty() {
        if widgets::empty_state(
            ui,
            theme,
            Icon::Search,
            "Nothing matches",
            &format!("No app matches “{}”.", state.search.trim()),
            Some(("Clear the search", Icon::Close)),
        )
        .is_some()
        {
            state.search.clear();
        }
        return;
    }

    let tracked = match world.apps.len() {
        1 => "1 repository tracked".to_owned(),
        count => format!("{count} repositories tracked"),
    };
    views::page_header(
        ui,
        theme,
        "Library",
        &format!(
            "{tracked}. The launcher reads their public releases; nothing is sent anywhere, \
             and no account is needed."
        ),
    );

    match state.view {
        LibraryView::Grid => grid(ui, theme, &cards, state, actions),
        LibraryView::List => list(ui, theme, &cards, state, actions),
    }
}

/// The cards, in as many columns as the window allows.
fn grid(ui: &mut egui::Ui, theme: &Theme, cards: &[&AppCard], state: &State, actions: &mut Vec<Action>) {
    // Cards are placed by hand rather than with a grid, so that the gaps are the ones the
    // arithmetic above assumed. `horizontal` adds the style's own item spacing between
    // children *on top of* any explicit space, and those extra eight points per gap are
    // what used to push the last card of a row past the right margin — the row was
    // `8 × (columns - 1)` wider than the width the columns were computed from.
    let gap = SPACE_LG;
    let available = ui.available_width();
    let columns = (((available + gap) / (CARD_MIN_WIDTH + gap)).floor() as usize).clamp(1, 4);
    let width = ((available - gap * columns.saturating_sub(1) as f32) / columns as f32)
        .floor()
        // Never ask for more than there is. With one narrow column the subtraction above
        // hands back the whole width, and a floor applied afterwards could only overflow.
        .min(available)
        .max(1.0);

    let rows = cards.len().div_ceil(columns);
    for (row, chunk) in cards.chunks(columns).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (index, card) in chunk.iter().enumerate() {
                if index > 0 {
                    ui.add_space(gap);
                }
                card_widget(
                    ui,
                    theme,
                    card,
                    width,
                    state.selected.as_deref() == Some(card.id()),
                    actions,
                );
            }
        });
        // No trailing gap after the last row: it would be space the scroll area counts
        // and nothing ever occupies.
        if row + 1 < rows {
            ui.add_space(gap);
        }
    }
}

/// The rows: one app per row, the same columns in the same place down the page.
fn list(ui: &mut egui::Ui, theme: &Theme, cards: &[&AppCard], state: &State, actions: &mut Vec<Action>) {
    let rows = cards.len();
    for (index, card) in cards.iter().enumerate() {
        row_widget(
            ui,
            theme,
            card,
            state.selected.as_deref() == Some(card.id()),
            actions,
        );
        if index + 1 < rows {
            ui.add_space(SPACE_SM);
        }
    }
}

/// One card in the grid.
fn card_widget(
    ui: &mut egui::Ui,
    theme: &Theme,
    card: &AppCard,
    width: f32,
    selected: bool,
    actions: &mut Vec<Action>,
) {
    // The background is allocated first, so the buttons placed inside it are
    // registered later and therefore win the click.
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_HEIGHT), Sense::click());

    if ui.is_rect_visible(rect) {
        let border = if selected {
            theme.palette.accent
        } else {
            theme.palette.border_subtle
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(RADIUS_LG),
            theme.palette.bg_elevated,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
    }

    let padding = SPACE_LG;
    let content_rect = rect.shrink(padding);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );
    card_contents(&mut child, theme, card, content_rect.width(), actions);

    if response.hovered() && !selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(RADIUS_LG),
            Stroke::new(1.0, theme.palette.border_strong),
            StrokeKind::Inside,
        );
    }

    if response.clicked() {
        actions.push(Action::Select(card.id().to_owned()));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// One row in the list.
fn row_widget(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, selected: bool, actions: &mut Vec<Action>) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());

    if ui.is_rect_visible(rect) {
        let border = if selected {
            theme.palette.accent
        } else {
            theme.palette.border_subtle
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(RADIUS_MD),
            theme.palette.bg_elevated,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
    }

    // Two exact regions rather than one layout, so the action keeps the same position in
    // every row whatever its label is. "Launch" and "Repository" are 54 points apart, and
    // a row whose button moved because of its own text would defeat the point of a list.
    let content = rect.shrink2(vec2(SPACE_LG, SPACE_MD));
    let action_width = ACTION_COLUMN_WIDTH.min((content.width() * 0.4).max(96.0));
    let text_rect = egui::Rect::from_min_max(
        content.min,
        egui::pos2(content.right() - action_width - SPACE_MD, content.bottom()),
    );
    let action_rect = egui::Rect::from_min_max(
        egui::pos2(content.right() - action_width, content.top()),
        content.max,
    );

    let mut text = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    let colour = egui::Color32::from_rgb(card.entry.color.r, card.entry.color.g, card.entry.color.b);
    widgets::brand_tile(&mut text, 32.0, colour, &card.entry.monogram());
    text.add_space(SPACE_MD);

    let column_width = (text_rect.width() - 32.0 - SPACE_MD).max(1.0);
    text.vertical(|ui| {
        ui.set_width(column_width);
        ui.add(
            egui::Label::new(
                RichText::new(&card.entry.name)
                    .font(theme.text.label.clone())
                    .color(theme.palette.text_primary),
            )
            .wrap_mode(egui::TextWrapMode::Truncate),
        );
        ui.add_space(SPACE_XS);
        ui.horizontal(|ui| {
            let (label, tone) = describe(&card.status);
            widgets::badge(ui, theme, label, tone);
            if let Some(version) = card.shown_version() {
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(version)
                        .font(theme.text.mono.clone())
                        .color(theme.palette.text_tertiary),
                );
            }
        });
    });

    let mut action = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(action_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    views::primary_button(&mut action, theme, card, actions);

    if response.hovered() && !selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(RADIUS_MD),
            Stroke::new(1.0, theme.palette.border_strong),
            StrokeKind::Inside,
        );
    }
    if response.clicked() {
        actions.push(Action::Select(card.id().to_owned()));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// Everything drawn inside a card.
fn card_contents(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, width: f32, actions: &mut Vec<Action>) {
    ui.set_width(width);
    ui.set_min_height(CARD_HEIGHT - SPACE_LG * 2.0);

    let colour = egui::Color32::from_rgb(card.entry.color.r, card.entry.color.g, card.entry.color.b);

    ui.horizontal(|ui| {
        widgets::brand_tile(ui, 40.0, colour, &card.entry.monogram());
        ui.add_space(SPACE_MD);
        ui.vertical(|ui| {
            ui.set_width(width - 40.0 - SPACE_MD);
            ui.add(
                egui::Label::new(
                    RichText::new(&card.entry.name)
                        .font(theme.text.heading.clone())
                        .color(theme.palette.text_primary),
                )
                .wrap_mode(egui::TextWrapMode::Truncate),
            );
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(&card.entry.tagline)
                        .font(theme.text.small.clone())
                        .color(theme.palette.text_secondary),
                )
                .wrap_mode(egui::TextWrapMode::Truncate),
            );
        });
    });

    ui.add_space(SPACE_MD);

    let (label, tone) = describe(&card.status);
    ui.horizontal(|ui| {
        widgets::badge(ui, theme, label, tone);
        if let Some(version) = card.shown_version() {
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new(version)
                    .font(theme.text.mono.clone())
                    .color(theme.palette.text_tertiary),
            );
        }
    });

    ui.add_space(SPACE_SM);
    ui.add(
        egui::Label::new(
            RichText::new(explain(card))
                .font(theme.text.small.clone())
                .color(theme.palette.text_tertiary),
        )
        .wrap_mode(egui::TextWrapMode::Truncate),
    );

    // Pin the action row to the bottom of the card, keeping a margin under it. This takes
    // up the slack explicitly rather than asking egui for a bottom-up layout: the slack is
    // known exactly, and a card is a fixed-size box, so there is nothing to negotiate.
    //
    // The margin is the point of it. Reserving only the row's own height put its lower
    // edge one padding step above the border, which is the same distance as the sides —
    // but a button has a border of its own and reads as closer than it measures, so the
    // row gets `SPACE_MD` of clearance as well.
    let slack = ui.available_height() - ACTION_ROW_HEIGHT - SPACE_MD;
    if slack > 0.0 {
        ui.add_space(slack);
    }

    ui.horizontal(|ui| {
        views::primary_button(ui, theme, card, actions);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::icon_button(ui, theme, Icon::ChevronRight, "Details", true).clicked() {
                actions.push(Action::Select(card.id().to_owned()));
            }
        });
    });
}
