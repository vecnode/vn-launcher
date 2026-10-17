//! Preferences.
//!
//! Two absences here are deliberate and stated on the page: there is no token field
//! and no sign-in, because the launcher only ever reads public repositories as an
//! anonymous guest, and `docs/RELEASES.md` explains why installing is not offered
//! yet.
//!
//! There is also no update *schedule*. The launcher never asks about its own
//! releases in the background; there is one button, and it is pressed on purpose.

use egui::{CornerRadius, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::Ui as State;
use crate::icons::Icon;
use crate::model::{Action, LauncherUpdate, Tone, World};
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
    releases(ui, theme, world, state, actions);
    ui.add_space(SPACE_XL);
    storage(ui, theme, world, actions);
    ui.add_space(SPACE_XL);
    about(ui, theme, world, actions);
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

/// What is read, and how it is cached.
fn releases(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    widgets::card(ui, theme, |ui| {
        widgets::section(
            ui,
            theme,
            "Releases",
            Some(
                "Nothing is read on a schedule. A repository is asked when you press \
                 Check for updates, and the answer is kept so that asking twice in a row \
                 does not spend a second request.",
            ),
        );

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

/// The launcher's own version, and the one button that asks about a newer one.
fn about(ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
    widgets::card(ui, theme, |ui| {
        widgets::section(ui, theme, "vn-launcher", None);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                widgets::overline(ui, theme, "Version");
                ui.add_space(2.0);
                ui.label(
                    RichText::new(vnl_core::VERSION)
                        .font(theme.text.mono.clone())
                        .color(theme.palette.text_primary),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let checking = world.update.is_checking();
                let response = widgets::button(
                    ui,
                    theme,
                    ButtonKind::Primary,
                    Some(Icon::Download),
                    "Update",
                    !checking && !world.offline,
                );
                if response.clicked() {
                    actions.push(Action::CheckLauncherUpdate);
                }
                if world.offline {
                    response.on_hover_text("Offline mode is on, so GitHub will not be asked");
                } else if checking {
                    response.on_hover_text("Already asking GitHub");
                } else {
                    response.on_hover_text("Ask whether a newer release of vn-launcher exists");
                }
            });
        });

        ui.add_space(SPACE_MD);
        let (icon, tone, message) = describe_update(&world.update);
        widgets::icon_label(ui, theme, icon, &message, tone);

        if let LauncherUpdate::Available { url, .. } = &world.update
            && widgets::button(
                ui,
                theme,
                ButtonKind::Secondary,
                Some(Icon::External),
                "Open the release page",
                true,
            )
            .clicked()
        {
            actions.push(Action::OpenUrl(url.clone()));
        }

        ui.add_space(SPACE_LG);
        widgets::separator(ui, theme);

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

        // Only what the launcher can be held to. An earlier version of this card also
        // promised what it would "never ask you for", which is a claim about a future
        // nobody has written yet rather than a fact about this build.
        ui.add_space(SPACE_LG);
        widgets::overline(ui, theme, "What is not switched on yet");
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "Downloading, installing and updating apps. The three arrive together once \
                 every repository publishes releases the same way — docs/INSTALL.md is the \
                 plan, docs/RELEASES.md is the agreement it waits on. Removing an app works \
                 today: it takes the repository out of the launcher and deletes what the \
                 launcher itself installed, and nothing else.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );
    });
}

/// How the launcher's own update check should read.
fn describe_update(update: &LauncherUpdate) -> (Icon, Tone, String) {
    match update {
        LauncherUpdate::Unknown => (
            Icon::Info,
            Tone::Neutral,
            "Press Update to ask whether a newer release exists.".to_owned(),
        ),
        LauncherUpdate::Checking => (Icon::Refresh, Tone::Info, "Asking GitHub…".to_owned()),
        LauncherUpdate::UpToDate => (
            Icon::Check,
            Tone::Success,
            format!("{} is the newest release.", vnl_core::VERSION),
        ),
        LauncherUpdate::Available { version, .. } => (
            Icon::Download,
            Tone::Accent,
            format!("vn-launcher {version} has been released."),
        ),
        LauncherUpdate::NoReleases => (
            Icon::Info,
            Tone::Neutral,
            "This repository publishes no releases yet, so there is nothing to compare \
             against."
                .to_owned(),
        ),
        LauncherUpdate::Failed(reason) => (Icon::Alert, Tone::Danger, format!("Could not check: {reason}")),
    }
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
