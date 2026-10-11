//! Adding a repository.
//!
//! This is the surface that makes the launcher extensible without a rebuild: paste
//! a repository, see what the launcher found there — how many releases, and which
//! file it would pick for *this* machine — and only then commit to it.

use egui::{Margin, RichText};

use crate::app::Ui as State;
use crate::icons::Icon;
use crate::model::{Action, World};
use crate::theme::{SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS, Theme};
use crate::views;
use crate::widgets::{self, ButtonKind};

/// Draw the add-repository surface.
pub fn show(ui: &mut egui::Ui, theme: &Theme, world: &World, state: &mut State, actions: &mut Vec<Action>) {
    views::page_header(
        ui,
        theme,
        "Add a repository",
        "Point the launcher at any public GitHub repository. No account, token or sign-in is \
         involved — the same public API a browser uses is all that is read.",
    );

    input_row(ui, theme, state, actions);
    ui.add_space(SPACE_XL);

    if state.probing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new(format!("Asking GitHub about {}…", state.add_input.trim()))
                    .font(theme.text.body.clone())
                    .color(theme.palette.text_secondary),
            );
        });
        ui.add_space(SPACE_XL);
    }

    if let Some(report) = state.probe.clone() {
        report_card(ui, theme, &report, state, actions);
        ui.add_space(SPACE_XL);
    }

    how_it_works(ui, theme, world);
}

/// The text field and the button that starts a check.
fn input_row(ui: &mut egui::Ui, theme: &Theme, state: &mut State, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut state.add_input)
            .hint_text("owner/repository  or  https://github.com/owner/repository")
            .desired_width(460.0)
            .font(theme.text.body.clone())
            .margin(Margin::symmetric(SPACE_MD as i8, 8));
        let response = ui.add(field);

        let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let ready = !state.add_input.trim().is_empty() && !state.probing;

        ui.add_space(SPACE_SM);
        let clicked =
            widgets::button(ui, theme, ButtonKind::Primary, Some(Icon::Search), "Check", ready).clicked();

        if ready && (clicked || submitted) {
            actions.push(Action::CheckRepository(state.add_input.clone()));
        }
    });

    if !state.add_input.trim().is_empty() {
        ui.add_space(SPACE_SM);
        match vnl_core::Repo::parse(&state.add_input) {
            Ok(repo) => widgets::icon_label(
                ui,
                theme,
                Icon::Check,
                &format!("Reads as {}", repo.slug()),
                crate::model::Tone::Success,
            ),
            Err(error) => widgets::icon_label(
                ui,
                theme,
                Icon::Alert,
                &error.to_string(),
                crate::model::Tone::Warning,
            ),
        }
    }
}

/// What the probe found.
fn report_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    report: &crate::model::ProbeReport,
    state: &mut State,
    actions: &mut Vec<Action>,
) {
    widgets::card(ui, theme, |ui| {
        ui.horizontal(|ui| {
            widgets::badge(
                ui,
                theme,
                if report.ok {
                    "Looks usable"
                } else {
                    "Nothing to install"
                },
                if report.ok {
                    crate::model::Tone::Success
                } else {
                    crate::model::Tone::Warning
                },
            );
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new(&report.repo)
                    .font(theme.text.mono.clone())
                    .color(theme.palette.text_primary),
            );
        });

        ui.add_space(SPACE_MD);
        ui.label(
            RichText::new(&report.summary)
                .font(theme.text.body.clone())
                .color(theme.palette.text_secondary),
        );

        ui.add_space(SPACE_LG);
        widgets::field(
            ui,
            theme,
            "Releases published",
            &report.release_count.to_string(),
            false,
        );
        ui.add_space(SPACE_MD);
        widgets::field(
            ui,
            theme,
            "Would be chosen for this system",
            report.matched_asset.as_deref().unwrap_or("nothing"),
            true,
        );
        ui.add_space(SPACE_MD);
        widgets::field(
            ui,
            theme,
            "Recent tags",
            &if report.recent_tags.is_empty() {
                "—".to_owned()
            } else {
                report.recent_tags.join(", ")
            },
            true,
        );

        ui.add_space(SPACE_LG);
        widgets::separator(ui, theme);
        ui.add_space(SPACE_LG);

        // The second half of the flow. The repository says where the code comes from; this
        // says where it already is on this machine, which is what the launcher can start.
        widgets::overline(ui, theme, "Where it already is on this machine");
        ui.add_space(SPACE_SM);
        let folder = egui::TextEdit::singleline(&mut state.local_input)
            .hint_text("optional — the folder you cloned or built it in")
            .desired_width(f32::INFINITY)
            .font(theme.text.body.clone())
            .margin(Margin::symmetric(SPACE_MD as i8, 8));
        ui.add(folder);
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "Empty is a fine answer: the launcher then looks for the app on this machine's \
                 PATH instead, as it does today. Reading the folder to find the binary inside \
                 it comes later.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_tertiary),
        );

        ui.add_space(SPACE_LG);
        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                theme,
                ButtonKind::Primary,
                Some(Icon::Plus),
                "Add to the launcher",
                report.ok,
            )
            .clicked()
            {
                actions.push(Action::AddRepository {
                    repo: report.repo.clone(),
                    local: state.local_input.clone(),
                });
            }
            if widgets::button(ui, theme, ButtonKind::Ghost, Some(Icon::Close), "Discard", true).clicked() {
                actions.push(Action::ResetProbe);
            }
        });
    });
}

/// What a repository entry can say, and what the launcher assumes without it.
fn how_it_works(ui: &mut egui::Ui, theme: &Theme, world: &World) {
    widgets::card(ui, theme, |ui| {
        widgets::overline(ui, theme, "What the launcher works out for itself");
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "The repository name, the release tags, which attached file belongs on this \
                 machine, and whether the app is already installed. An entry is therefore \
                 usually just an id, a name and a repository.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );

        ui.add_space(SPACE_LG);
        widgets::overline(ui, theme, "What you can add when the guesses are wrong");
        ui.add_space(SPACE_SM);
        ui.label(
            RichText::new(
                "stems — the file-name prefixes this repository's assets use, when they are not \
                 named after the repository.\nextensions — which archive kinds to accept, in \
                 order of preference.\nbinary — the executable inside the archive.\ncolour — \
                 the brand colour for the app's tile.\nlaunch — the command that starts it.",
            )
            .font(theme.text.mono.clone())
            .color(theme.palette.text_tertiary),
        );

        ui.add_space(SPACE_LG);
        widgets::overline(ui, theme, "Where your entries are kept");
        ui.add_space(SPACE_XS);
        ui.label(
            RichText::new(&world.user_catalog)
                .font(theme.text.mono.clone())
                .color(theme.palette.text_primary),
        );
        ui.add_space(SPACE_XS);
        ui.label(
            RichText::new(
                "Your entries are merged over the ones the launcher ships with, so an update \
                 never overwrites what you added.",
            )
            .font(theme.text.small.clone())
            .color(theme.palette.text_secondary),
        );
    });
}
