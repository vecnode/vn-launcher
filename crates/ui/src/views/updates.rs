//! The apps that have something to do.

use egui::{Color32, CornerRadius, RichText, Sense, Stroke, StrokeKind, vec2};

use crate::app::Ui as State;
use crate::icons::Icon;
use crate::model::{Action, AppCard, World, describe, explain};
use crate::theme::{RADIUS_MD, SPACE_LG, SPACE_MD, SPACE_SM, Theme};
use crate::views;
use crate::widgets;

/// Draw the updates surface.
pub fn show(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    if world.apps.is_empty() {
        widgets::empty_state(
            ui,
            theme,
            Icon::Package,
            "No apps are configured",
            "The catalog is empty. Add a repository to get started.",
            Some(("Add a repository", Icon::Plus)),
        );
        return;
    }

    let attention = world.attention();
    views::page_header(
        ui,
        theme,
        "Updates",
        if attention.is_empty() {
            "Nothing needs your attention."
        } else {
            "These apps have something waiting. Nothing is downloaded until installing is switched on."
        },
    );

    if attention.is_empty() {
        widgets::card(ui, theme, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                crate::icons::paint(ui.painter(), Icon::Check, rect, theme.palette.success, 1.8);
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new("Nothing to update")
                        .font(theme.text.heading.clone())
                        .color(theme.palette.text_primary),
                );
            });
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new(
                    "Every repository the launcher tracks either matches what is installed, or \
                     publishes nothing to install yet. Apps that have never been installed are \
                     not updates, so they are not counted here — you will find them in the \
                     Library.",
                )
                .font(theme.text.small.clone())
                .color(theme.palette.text_secondary),
            );
        });
        return;
    }

    for card in attention {
        row(ui, theme, card, actions);
        ui.add_space(SPACE_SM);
    }

    ui.add_space(SPACE_MD);
    ui.label(
        RichText::new(
            "Updating is not switched on yet: it waits on every repository publishing releases \
             the same way, so that one set of rules is correct everywhere.",
        )
        .font(theme.text.small.clone())
        .color(theme.palette.text_tertiary),
    );
    let _ = state;
}

/// One row of the updates list.
fn row(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, actions: &mut Vec<Action>) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::click());

    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            CornerRadius::same(RADIUS_MD),
            theme.palette.bg_elevated,
            Stroke::new(1.0, theme.palette.border_subtle),
            StrokeKind::Inside,
        );
    }

    let content = rect.shrink(SPACE_LG);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    child.horizontal(|ui| {
        let colour = Color32::from_rgb(card.entry.color.r, card.entry.color.g, card.entry.color.b);
        widgets::brand_tile(ui, 40.0, colour, &card.entry.monogram());
        ui.add_space(SPACE_MD);

        let block = ui.vertical(|ui| {
            // The tile and its spacing are already placed, so only the remainder is
            // available for the text column.
            ui.set_width((ui.available_width() - SPACE_SM).max(120.0));
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&card.entry.name)
                        .font(theme.text.heading.clone())
                        .color(theme.palette.text_primary),
                );
                ui.add_space(SPACE_SM);
                let (label, tone) = describe(&card.status);
                widgets::badge(ui, theme, label, tone);
            });
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(explain(card))
                        .font(theme.text.small.clone())
                        .color(theme.palette.text_secondary),
                )
                .wrap_mode(egui::TextWrapMode::Truncate),
            );
        });
        block.response.on_hover_cursor(egui::CursorIcon::PointingHand);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            views::primary_button(ui, theme, card, actions);
        });
    });

    if response.hovered() {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(RADIUS_MD),
            Stroke::new(1.0, theme.palette.border_strong),
            StrokeKind::Inside,
        );
    }
    if response.clicked() {
        actions.push(Action::Select(card.id().to_owned()));
        actions.push(Action::SetTab(crate::model::Tab::Library));
    }
}
