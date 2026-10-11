//! The five surfaces.
//!
//! Each is a plain function of the [`World`] and the interface's own state: it
//! draws, and it appends [`Action`]s. None of them performs IO.
//!
//! [`World`]: crate::model::World
//! [`Action`]: crate::model::Action

pub mod detail;
pub mod discover;
pub mod library;
pub mod settings;
pub mod updates;

use crate::icons::Icon;
use crate::model::{Action, AppCard, World, deferred_reason};
use crate::theme::{SPACE_XL, SPACE_XS, Theme};
use crate::widgets::{self, ButtonKind};

/// What a card's one button should say and do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primary {
    /// Start it.
    Launch,
    /// Install it, once installation exists.
    Install,
    /// Update it, once updating exists.
    Update,
    /// Look it up, because nothing is known about it yet.
    Check,
    /// Look it up again, because the last attempt failed.
    Retry,
    /// Open the repository, because there is nothing to install.
    Repository,
}

/// The label and mark for a primary action.
#[must_use]
pub fn primary_label(action: Primary) -> (&'static str, Icon) {
    match action {
        Primary::Launch => ("Launch", Icon::Play),
        Primary::Install => ("Install", Icon::Download),
        Primary::Update => ("Update", Icon::Download),
        Primary::Check => ("Check", Icon::Refresh),
        Primary::Retry => ("Try again", Icon::Refresh),
        Primary::Repository => ("Repository", Icon::External),
    }
}

/// The one thing worth doing with this app right now.
#[must_use]
pub fn primary_for(card: &AppCard) -> Primary {
    use vnl_core::status::AppStatus;
    match &card.status {
        AppStatus::CheckFailed { .. } => Primary::Retry,
        AppStatus::Available { .. } => Primary::Install,
        AppStatus::UpdateAvailable { .. } => Primary::Update,
        AppStatus::Unknown | AppStatus::Checking => Primary::Check,
        AppStatus::UpToDate | AppStatus::Found { .. } | AppStatus::InstalledUncomparable { .. } => {
            if can_launch(card) {
                Primary::Launch
            } else {
                Primary::Repository
            }
        }
        AppStatus::Link
        | AppStatus::NoReleases
        | AppStatus::NoBuildForThisSystem { .. }
        | AppStatus::AmbiguousBuild { .. } => Primary::Repository,
    }
}

/// Whether a primary action can be taken right now.
///
/// Installation and removal are deliberately not offered yet — see
/// `docs/RELEASES.md` — so their buttons are shown disabled rather than hidden: an
/// app with no action at all looks broken, whereas a disabled button with a reason
/// on hover explains the state of the world.
#[must_use]
pub fn primary_enabled(action: Primary, card: &AppCard) -> bool {
    match action {
        Primary::Launch => can_launch(card),
        Primary::Install | Primary::Update => false,
        Primary::Check => !card.status.is_busy(),
        Primary::Retry | Primary::Repository => true,
    }
}

/// Whether the launcher can start this app.
///
/// It must declare how to start, and something must actually be on the machine: a
/// launcher that offers to launch an app it cannot find is worse than one that says
/// nothing.
#[must_use]
pub fn can_launch(card: &AppCard) -> bool {
    use vnl_core::status::AppStatus;
    !card.entry.launch.is_empty()
        && (card.install.is_managed() || matches!(card.status, AppStatus::Found { .. }))
}

/// Draw an app's leading button, and record what it asked for.
///
/// One implementation for the library grid, the detail page and the updates list,
/// so the three can never disagree about what an app offers.
pub fn primary_button(ui: &mut egui::Ui, theme: &Theme, card: &AppCard, actions: &mut Vec<Action>) {
    let primary = primary_for(card);
    let (label, icon) = primary_label(primary);
    let enabled = primary_enabled(primary, card);
    let kind = if primary == Primary::Launch {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };

    let response = widgets::button(ui, theme, kind, Some(icon), label, enabled);
    if response.clicked() {
        push_primary(primary, card, actions);
    }

    match primary {
        Primary::Install | Primary::Update => {
            let topic = if primary == Primary::Install {
                "install"
            } else {
                "update"
            };
            response.on_hover_text(deferred_reason(topic));
        }
        Primary::Check => {
            response.on_hover_text("Ask GitHub what this repository publishes");
        }
        Primary::Retry => {
            response.on_hover_text("Ask GitHub again");
        }
        Primary::Launch | Primary::Repository => {}
    }
}

/// The apps matching the current search text.
#[must_use]
pub fn filtered<'a>(world: &'a World, query: &str) -> Vec<&'a AppCard> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return world.apps.iter().collect();
    }
    world
        .apps
        .iter()
        .filter(|card| {
            card.entry.name.to_lowercase().contains(&query)
                || card.entry.id.contains(&query)
                || card.entry.tagline.to_lowercase().contains(&query)
                || card.entry.repo.slug().contains(&query)
        })
        .collect()
}

/// A page header: a title and an explanation.
pub fn page_header(ui: &mut egui::Ui, theme: &Theme, title: &str, detail: &str) {
    ui.label(
        egui::RichText::new(title)
            .font(theme.text.display.clone())
            .color(theme.palette.text_primary),
    );
    if !detail.is_empty() {
        ui.add_space(SPACE_XS);
        ui.label(
            egui::RichText::new(detail)
                .font(theme.text.body.clone())
                .color(theme.palette.text_secondary),
        );
    }
    ui.add_space(SPACE_XL);
}

/// Record the action a card's button implies.
pub fn push_primary(action: Primary, card: &AppCard, actions: &mut Vec<Action>) {
    match action {
        Primary::Launch => actions.push(Action::Launch(card.id().to_owned())),
        Primary::Check | Primary::Retry => {
            actions.push(Action::RefreshApp(card.id().to_owned()));
        }
        Primary::Install | Primary::Update | Primary::Repository => {
            actions.push(Action::OpenRepository(card.id().to_owned()));
        }
    }
}
