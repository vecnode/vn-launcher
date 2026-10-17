//! One app, in full.
//!
//! The detail page is where the launcher is honest about what it knows and what it
//! does not: which file it would take for this machine, what it looked at when
//! looking for an existing installation, and exactly why installing is not offered
//! yet.

use egui::{Color32, Margin, RichText, Stroke};
use vnl_core::status::AppStatus;
use vnl_core::time;

use crate::app::{Ui as State, now_unix};
use crate::icons::Icon;
use crate::model::{Action, AppCard, World, describe, explain};
use crate::theme::{SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS, Theme};
use crate::views::{self, Primary};
use crate::widgets::{self, ButtonKind};

/// Draw the detail page for the selected app.
pub fn show(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    let Some(id) = state.selected.clone() else {
        return;
    };
    let Some(card) = world.app(&id) else {
        return;
    };

    if widgets::button(
        ui,
        theme,
        ButtonKind::Ghost,
        Some(Icon::ChevronLeft),
        "Library",
        true,
    )
    .clicked()
    {
        actions.push(Action::Back);
    }

    ui.add_space(SPACE_LG);
    header(ui, theme, card);
    ui.add_space(SPACE_LG);
    action_row(ui, theme, card, state, actions);
    ui.add_space(SPACE_XL);

    status_card(ui, theme, card);
    ui.add_space(SPACE_LG);
    facts_card(ui, theme, card, world);
    ui.add_space(SPACE_LG);

    // A let-chain rather than nested `if let`s: both conditions are needed together,
    // and the notes only make sense attached to the release that carries them.
    if let Some(release) = &card.release
        && let Some(notes) = &release.notes
    {
        notes_card(ui, theme, release, notes, actions);
        ui.add_space(SPACE_LG);
    }

    if !card.detection_checked.is_empty() {
        searched_card(ui, theme, card);
        ui.add_space(SPACE_LG);
    }

    pending_card(ui, theme, card, actions);
}

/// The tile, the name and the one-line description.
fn header(ui: &mut egui::Ui, theme: &Theme, card: &AppCard) {
    ui.horizontal(|ui| {
        let colour = Color32::from_rgb(card.entry.color.r, card.entry.color.g, card.entry.color.b);
        widgets::brand_tile(ui, 56.0, colour, &card.entry.monogram());
        ui.add_space(SPACE_LG);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&card.entry.name)
                    .font(theme.text.display.clone())
                    .color(theme.palette.text_primary),
            );
            ui.add_space(SPACE_XS);
            ui.label(
                RichText::new(&card.entry.tagline)
                    .font(theme.text.body.clone())
                    .color(theme.palette.text_secondary),
            );
        });
    });
}

/// Everything that can be done with this app.
fn action_row(
    ui: &mut egui::Ui,
    theme: &Theme,
    card: &AppCard,
    state: &mut State,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        views::primary_button(ui, theme, card, actions);

        // When the leading action is not Launch — an app waiting on an install, say —
        // Launch is still offered beside it if there is something to start.
        if views::primary_for(card) != Primary::Launch
            && views::can_launch(card)
            && widgets::button(ui, theme, ButtonKind::Primary, Some(Icon::Play), "Launch", true).clicked()
        {
            actions.push(Action::Launch(card.id().to_owned()));
        }

        if widgets::button(
            ui,
            theme,
            ButtonKind::Secondary,
            Some(Icon::External),
            "Repository",
            true,
        )
        .clicked()
        {
            actions.push(Action::OpenRepository(card.id().to_owned()));
        }

        if widgets::button(
            ui,
            theme,
            ButtonKind::Ghost,
            Some(Icon::Folder),
            "Install folder",
            true,
        )
        .clicked()
        {
            actions.push(Action::OpenInstallRoot);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Separated from the rest on purpose: this is the only action on the page that
            // destroys anything, and it sits where a hand is least likely to arrive by
            // accident — behind a confirmation either way.
            let remove = widgets::button(ui, theme, ButtonKind::Danger, Some(Icon::Trash), "Remove", true);
            if remove.clicked() {
                state.confirm_remove = Some(card.id().to_owned());
            }
        });
    });
}

/// The verdict, and the sentence that explains it.
fn status_card(ui: &mut egui::Ui, theme: &Theme, card: &AppCard) {
    widgets::card(ui, theme, |ui| {
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
        ui.label(
            RichText::new(explain(card))
                .font(theme.text.body.clone())
                .color(theme.palette.text_secondary),
        );

        if let Some(release) = &card.release {
            if let Some(published) = release.published_at.as_deref().and_then(time::parse_iso8601) {
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(format!(
                        "Published {} · {}",
                        time::format_date(published),
                        time::describe_age(published, now_unix())
                    ))
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_tertiary),
                );
            }
            if release.prerelease {
                ui.add_space(SPACE_SM);
                widgets::icon_label(
                    ui,
                    theme,
                    Icon::Alert,
                    "This release is marked a pre-release.",
                    crate::model::Tone::Warning,
                );
            }
        }
    });
}

/// The facts: repository, chosen asset, where things live.
fn facts_card(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, world: &World) {
    widgets::card(ui, theme, |ui| {
        widgets::overline(ui, theme, "Details");
        ui.add_space(SPACE_MD);

        fact(ui, theme, "Repository", &card.entry.repo.slug(), true);

        if let Some(release) = &card.release {
            fact(ui, theme, "Latest release", &release.tag, true);
            fact(
                ui,
                theme,
                "Attached files",
                &release.asset_count.to_string(),
                false,
            );
            match &release.asset {
                vnl_core::AssetChoice::Chosen(asset) => {
                    fact(
                        ui,
                        theme,
                        "Chosen for this system",
                        &format!("{} ({})", asset.name, asset.human_size()),
                        true,
                    );
                    fact(
                        ui,
                        theme,
                        "Verification",
                        if asset.digest.is_some() {
                            "GitHub publishes a SHA-256 digest for this file"
                        } else {
                            "No digest published; it would be recorded on first download"
                        },
                        false,
                    );
                }
                vnl_core::AssetChoice::None => fact(
                    ui,
                    theme,
                    "Chosen for this system",
                    "none — this release has no build for this system",
                    false,
                ),
                vnl_core::AssetChoice::Ambiguous(names) => fact(
                    ui,
                    theme,
                    "Chosen for this system",
                    &format!("ambiguous between {}", names.join(", ")),
                    false,
                ),
            }
        }

        fact(
            ui,
            theme,
            "Would install to",
            &format!("{}/{}", world.install_root, card.entry.id),
            true,
        );

        if let Some(path) = &card.detection_path {
            fact(ui, theme, "Found at", path, true);
        }

        let launch_command = if card.entry.launch.is_empty() {
            "not a program — installed into DeepSeek Harness".to_owned()
        } else {
            let os = world.target.map_or(vnl_core::Os::Linux, |target| target.os);
            card.entry.launch.for_os(os).join(" ")
        };
        fact(ui, theme, "How it starts", &launch_command, true);
    });
}

/// A labelled value inside a card, with the spacing that separates it from the next.
///
/// Stacked rather than laid out in a two-column grid. Several of these values are
/// long monospace paths, and a grid divides whatever width it is given between its
/// columns rather than asking for more — which wrapped them a few characters at a
/// time and made the page unreadable.
fn fact(ui: &mut egui::Ui, theme: &Theme, label: &str, value: &str, mono: bool) {
    widgets::field(ui, theme, label, value, mono);
    ui.add_space(SPACE_MD);
}
/// The release notes, kept as plain text.
fn notes_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    release: &vnl_core::ReleaseInfo,
    notes: &str,
    actions: &mut Vec<Action>,
) {
    widgets::card(ui, theme, |ui| {
        widgets::overline(ui, theme, &format!("Release notes · {}", release.tag));
        ui.add_space(SPACE_SM);
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(
                    RichText::new(notes)
                        .font(theme.text.small.clone())
                        .color(theme.palette.text_secondary),
                );
            });
        ui.add_space(SPACE_SM);
        if widgets::button(
            ui,
            theme,
            ButtonKind::Ghost,
            Some(Icon::External),
            "Open the release page",
            true,
        )
        .clicked()
            && !release.html_url.is_empty()
        {
            actions.push(Action::OpenUrl(release.html_url.clone()));
        }
    });
}

/// Every location that was examined when looking for an existing install.
fn searched_card(ui: &mut egui::Ui, theme: &Theme, card: &AppCard) {
    widgets::card(ui, theme, |ui| {
        widgets::overline(ui, theme, "Looked for it in");
        ui.add_space(SPACE_SM);
        for location in &card.detection_checked {
            ui.label(
                RichText::new(location)
                    .font(theme.text.mono.clone())
                    .color(theme.palette.text_tertiary),
            );
        }
        if matches!(card.status, AppStatus::Found { .. }) {
            ui.add_space(SPACE_SM);
            widgets::icon_label(
                ui,
                theme,
                Icon::Info,
                "The launcher did not install this, so it will not update or remove it.",
                crate::model::Tone::Info,
            );
        }
    });
}

/// Why installation is not offered yet.
fn pending_card(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, actions: &mut Vec<Action>) {
    if !matches!(
        card.status,
        AppStatus::Available { .. } | AppStatus::UpdateAvailable { .. }
    ) {
        return;
    }
    let stroke = Stroke::new(1.0, theme.palette.border_subtle);
    let width = (ui.available_width() - 2.0 * widgets::CARD_PADDING).max(160.0);
    let frame = egui::Frame::new()
        .fill(theme.palette.bg_panel)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(crate::theme::RADIUS_MD))
        .inner_margin(Margin::same(SPACE_LG as i8));
    frame.show(ui, |ui| {
        ui.set_width(width);
        widgets::icon_label(
            ui,
            theme,
            Icon::Info,
            "Downloading and unpacking is not switched on yet",
            crate::model::Tone::Info,
        );
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "The launcher can already read every repository's releases and work out which \
                 file belongs on this machine. Downloading, unpacking and recording the result \
                 arrive together — the plan is written down, and it waits on each repository \
                 publishing releases the same way.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );
        ui.add_space(SPACE_MD);
        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                theme,
                ButtonKind::Secondary,
                Some(Icon::External),
                "Read the install plan",
                true,
            )
            .clicked()
            {
                actions.push(Action::OpenUrl(crate::INSTALL_PLAN_URL.to_owned()));
            }
            if widgets::button(
                ui,
                theme,
                ButtonKind::Ghost,
                Some(Icon::External),
                "Install from the repository instead",
                true,
            )
            .clicked()
            {
                actions.push(Action::OpenDocs(card.id().to_owned()));
            }
        });
    });
}
