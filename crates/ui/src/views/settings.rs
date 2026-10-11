//! Preferences.
//!
//! Two absences here are deliberate and stated on the page: there is no token field
//! and no sign-in, because the launcher only ever reads public repositories as an
//! anonymous guest, and `docs/RELEASES.md` explains why installing is not offered
//! yet.

use egui::{CornerRadius, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::Ui as State;
use crate::icons::Icon;
use crate::model::{Action, World};
use crate::theme::{RADIUS_MD, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, Theme, ThemeChoice};
use crate::views;
use crate::widgets::{self, ButtonKind};

/// Draw the settings surface.
pub fn show(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    views::page_header(
        ui,
        theme,
        "Settings",
        "Everything the launcher keeps about itself. Your apps are not touched from here.",
    );

    appearance(ui, theme, state, actions);
    ui.add_space(SPACE_XL);
    checking(ui, theme, world, state, actions);
    ui.add_space(SPACE_XL);
    storage(ui, theme, world, actions);
    ui.add_space(SPACE_XL);
    about(ui, theme, world);
}

/// Theme choice.
fn appearance(ui: &mut egui::Ui, theme: &Theme, state: &mut State, actions: &mut Vec<Action>) {
    widgets::card(ui, theme, |ui| {
        widgets::section(ui, theme, "Appearance", None);
        ui.horizontal(|ui| {
            for choice in ThemeChoice::ALL {
                let selected = state.theme_choice == choice;
                if segmented(ui, theme, selected, choice.label()) {
                    state.theme_choice = choice;
                    actions.push(Action::SetTheme(choice));
                }
                ui.add_space(SPACE_SM);
            }
        });
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new("System follows whatever the operating system is set to.")
                .font(theme.text.small.clone())
                .color(theme.palette.text_tertiary),
        );
    });
}

/// When updates are looked for.
fn checking(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    widgets::card(ui, theme, |ui| {
        widgets::section(
            ui,
            theme,
            "Checking for updates",
            Some(
                "Each app's releases are read at most once per interval. The reading is cached, \
                 so a launcher left open does not keep asking.",
            ),
        );

        ui.horizontal(|ui| {
            for hours in [1_u32, 6, 12, 24, 168] {
                let selected = state.interval_hours == hours;
                let label = if hours >= 168 {
                    "Weekly".to_owned()
                } else if hours == 24 {
                    "Daily".to_owned()
                } else {
                    format!("{hours} h")
                };
                if segmented(ui, theme, selected, &label) {
                    state.interval_hours = hours;
                    actions.push(Action::SetIntervalHours(hours));
                }
                ui.add_space(SPACE_SM);
            }
        });

        ui.add_space(SPACE_LG);

        let mut prerelease = state.prerelease;
        if ui
            .checkbox(&mut prerelease, "Offer pre-releases as well as finished releases")
            .changed()
        {
            state.prerelease = prerelease;
            actions.push(Action::SetPrerelease(prerelease));
        }

        let mut offline = world.offline;
        if ui
            .checkbox(&mut offline, "Offline mode — never contact GitHub")
            .changed()
        {
            actions.push(Action::SetOffline(offline));
        }

        ui.add_space(SPACE_LG);
        widgets::separator(ui, theme);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Cached release feeds")
                        .font(theme.text.label.clone())
                        .color(theme.palette.text_primary),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("{} response(s) held locally.", world.cache_entries))
                        .font(theme.text.small.clone())
                        .color(theme.palette.text_secondary),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(
                    ui,
                    theme,
                    ButtonKind::Secondary,
                    Some(Icon::Trash),
                    "Forget cached feeds",
                    true,
                )
                .clicked()
                {
                    actions.push(Action::ClearCache);
                }
            });
        });
    });
}

/// Where things live.
fn storage(ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
    widgets::card(ui, theme, |ui| {
        widgets::section(ui, theme, "Storage", None);

        widgets::field(
            ui,
            theme,
            "Apps would be installed to",
            &format!("{}/<app>", world.install_root),
            true,
        );
        ui.add_space(SPACE_MD);
        widgets::field(ui, theme, "Your catalog entries", &world.user_catalog, true);
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(if world.user_catalog_exists {
                "This file exists and is merged over the built-in catalog."
            } else {
                "This file does not exist yet; the launcher is using only what it ships with."
            })
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );

        ui.add_space(SPACE_LG);
        if widgets::button(
            ui,
            theme,
            ButtonKind::Secondary,
            Some(Icon::Folder),
            "Open the install folder",
            true,
        )
        .clicked()
        {
            actions.push(Action::OpenInstallRoot);
        }
    });
}

/// Version, machine and licence.
fn about(ui: &mut egui::Ui, theme: &Theme, world: &World) {
    widgets::card(ui, theme, |ui| {
        widgets::section(ui, theme, "About", None);

        widgets::field(
            ui,
            theme,
            "Version",
            &format!("vn-launcher {}", vnl_core::VERSION),
            true,
        );
        ui.add_space(SPACE_MD);
        widgets::field(
            ui,
            theme,
            "This machine",
            &world
                .target
                .map_or_else(|| "unknown".to_owned(), |target| target.label()),
            false,
        );
        ui.add_space(SPACE_MD);
        widgets::field(ui, theme, "Licence", "MIT", false);
        ui.add_space(SPACE_MD);
        widgets::field(
            ui,
            theme,
            "Repositories tracked",
            &world.apps.len().to_string(),
            false,
        );

        ui.add_space(SPACE_LG);
        widgets::overline(ui, theme, "What this launcher will never ask you for");
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "An account, a token, a sign-in, or a payment. Every request goes to GitHub's \
                 public API as an anonymous guest, which is also why feeds are cached and read \
                 no more often than the interval above.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );

        ui.add_space(SPACE_LG);
        widgets::overline(ui, theme, "What is not switched on yet");
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "Downloading, installing, updating and removing. Those four arrive together once \
                 every repository publishes releases the same way — see docs/RELEASES.md.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );
    });
}

/// One option in a segmented control.
fn segmented(ui: &mut egui::Ui, theme: &Theme, selected: bool, label: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        theme.text.label.clone(),
        theme.palette.text_primary,
    );
    let size = vec2(galley.size().x + SPACE_LG * 2.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let (fill, stroke, foreground) = if selected {
            (theme.palette.accent, Stroke::NONE, theme.palette.on_accent)
        } else if response.hovered() {
            (
                theme.palette.bg_hover,
                Stroke::new(1.0, theme.palette.border_strong),
                theme.palette.text_primary,
            )
        } else {
            (
                theme.palette.bg_elevated,
                Stroke::new(1.0, theme.palette.border_subtle),
                theme.palette.text_secondary,
            )
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            CornerRadius::same(RADIUS_MD),
            fill,
            stroke,
            StrokeKind::Inside,
        );
        painter.galley(
            pos2(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            foreground,
        );
    }

    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if selected {
        response.on_hover_text("Selected")
    } else {
        response
    }
    .clicked()
}
