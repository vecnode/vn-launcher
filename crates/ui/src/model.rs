//! What the interface renders, and what it asks for.
//!
//! The split is deliberate. [`World`] is everything the shell knows, already
//! gathered; [`Action`] is everything a person can ask for, as a plain value. The
//! interface never performs IO, never reads a config file and never spawns a
//! process — it draws a [`World`] and returns a list of [`Action`]s, which the shell
//! then carries out. That is what makes the interface testable with a literal and
//! the shell testable with a fake.

use vnl_core::Target;
use vnl_core::catalog::AppEntry;
use vnl_core::status::{AppStatus, InstallState, ReleaseInfo};

use crate::icons::Icon;
use crate::theme::ThemeChoice;

/// A meaning the interface colours things by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Nothing to say.
    Neutral,
    /// The launcher's own accent.
    Accent,
    /// Something is well.
    Success,
    /// Something needs attention soon.
    Warning,
    /// Something is wrong.
    Danger,
    /// Neutral information.
    Info,
}

/// One app, as the interface sees it.
#[derive(Debug, Clone)]
pub struct AppCard {
    /// The catalog entry.
    pub entry: AppEntry,
    /// The verdict.
    pub status: AppStatus,
    /// What is on this machine.
    pub install: InstallState,
    /// Where it was found, when it was.
    pub detection_path: Option<String>,
    /// Every location that was looked at.
    pub detection_checked: Vec<String>,
    /// The release the verdict came from, when there was one.
    pub release: Option<ReleaseInfo>,
    /// When this app was last checked, in seconds since the epoch.
    pub checked_at: Option<i64>,
}

impl AppCard {
    /// The app's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.entry.id
    }

    /// Whether this app wants a person's attention.
    #[must_use]
    pub fn needs_attention(&self) -> bool {
        self.status.needs_attention()
    }

    /// The version to show, if any is known.
    #[must_use]
    pub fn shown_version(&self) -> Option<String> {
        self.install
            .version()
            .map(ToOwned::to_owned)
            .or_else(|| self.release.as_ref().map(|release| release.version.clone()))
    }
}

/// A message from the shell.
#[derive(Debug, Clone)]
pub struct Notice {
    /// How it should read.
    pub tone: Tone,
    /// The headline.
    pub title: String,
    /// The explanation, when there is one.
    pub detail: Option<String>,
}

impl Notice {
    /// An informational notice.
    #[must_use]
    pub fn info(title: impl Into<String>) -> Self {
        Self {
            tone: Tone::Info,
            title: title.into(),
            detail: None,
        }
    }

    /// A notice about something going wrong.
    #[must_use]
    pub fn error(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            tone: Tone::Danger,
            title: title.into(),
            detail: Some(detail.into()),
        }
    }

    /// A notice about something succeeding.
    #[must_use]
    pub fn success(title: impl Into<String>) -> Self {
        Self {
            tone: Tone::Success,
            title: title.into(),
            detail: None,
        }
    }

    /// Attach an explanation.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// What the add-repository flow learned about a repository.
#[derive(Debug, Clone)]
pub struct ProbeReport {
    /// The repository that was inspected, `owner/name`.
    pub repo: String,
    /// Whether it looks usable.
    pub ok: bool,
    /// One line saying what was found.
    pub summary: String,
    /// How many releases are published.
    pub release_count: usize,
    /// The most recent tags.
    pub recent_tags: Vec<String>,
    /// The attached file that would be chosen for this machine.
    pub matched_asset: Option<String>,
}

/// What the launcher knows about its own newer releases.
///
/// Nothing here happens on a schedule. The launcher never checks for its own updates
/// in the background; a person presses **Update** and this is the answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LauncherUpdate {
    /// Not asked yet.
    #[default]
    Unknown,
    /// An answer is on its way.
    Checking,
    /// This build is the newest release.
    UpToDate,
    /// A newer release exists.
    Available {
        /// The version on offer.
        version: String,
        /// Its release page.
        url: String,
    },
    /// The launcher's own repository publishes no releases yet, so there is nothing
    /// to compare against.
    NoReleases,
    /// The question could not be answered.
    Failed(String),
}

impl LauncherUpdate {
    /// Whether a check is in flight.
    #[must_use]
    pub fn is_checking(&self) -> bool {
        matches!(self, Self::Checking)
    }
}

/// Everything the interface draws.
#[derive(Debug, Clone, Default)]
pub struct World {
    /// The apps, in display order.
    pub apps: Vec<AppCard>,
    /// Messages to show.
    pub notices: Vec<Notice>,
    /// The machine, once known.
    pub target: Option<Target>,
    /// Whether a check is running.
    pub busy: bool,
    /// When the last full check finished, in seconds since the epoch.
    pub checked_at: Option<i64>,
    /// Whether network reads are suppressed.
    pub offline: bool,
    /// How many responses the feed cache holds.
    pub cache_entries: usize,
    /// Where apps will be installed.
    pub install_root: String,
    /// The user's catalog file.
    pub user_catalog: String,
    /// Whether the user catalog exists yet.
    pub user_catalog_exists: bool,
    /// What is known about the launcher's own newer releases.
    pub update: LauncherUpdate,
}

impl World {
    /// One app by id.
    #[must_use]
    pub fn app(&self, id: &str) -> Option<&AppCard> {
        self.apps.iter().find(|card| card.id() == id)
    }

    /// The apps that want attention.
    #[must_use]
    pub fn attention(&self) -> Vec<&AppCard> {
        self.apps.iter().filter(|card| card.needs_attention()).collect()
    }

    /// How many apps want attention.
    #[must_use]
    pub fn attention_count(&self) -> usize {
        self.apps.iter().filter(|card| card.needs_attention()).count()
    }
}

/// A destination in the navigation rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// The apps.
    Library,
    /// The apps with something to do.
    Updates,
    /// Add a repository.
    Discover,
    /// Preferences and about.
    Settings,
}

impl Tab {
    /// Every destination, in rail order.
    pub const ALL: [Self; 4] = [Self::Library, Self::Updates, Self::Discover, Self::Settings];

    /// How it is written.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Updates => "Updates",
            Self::Discover => "Add repository",
            Self::Settings => "Settings",
        }
    }

    /// The mark beside it.
    #[must_use]
    pub fn icon(self) -> Icon {
        match self {
            Self::Library => Icon::Package,
            Self::Updates => Icon::Refresh,
            Self::Discover => Icon::Plus,
            Self::Settings => Icon::Sliders,
        }
    }
}

/// Something a person asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Check every app against GitHub.
    Refresh,
    /// Check one app.
    RefreshApp(String),
    /// Open an app's detail page.
    Select(String),
    /// Leave the detail page.
    Back,
    /// Move to a destination.
    SetTab(Tab),
    /// Change the search text.
    SetSearch(String),
    /// Change the theme.
    SetTheme(ThemeChoice),
    /// Include or exclude pre-releases.
    SetPrerelease(bool),
    /// Ask whether the launcher itself has a newer release.
    ///
    /// Deliberately the only way that question is ever asked: there is no schedule
    /// and no automatic check.
    CheckLauncherUpdate,
    /// Turn network reads off or on.
    SetOffline(bool),
    /// Forget every cached feed.
    ClearCache,
    /// Start an installed app.
    Launch(String),
    /// Open an app's repository.
    OpenRepository(String),
    /// Open an app's home page.
    OpenHomepage(String),
    /// Open an app's install instructions.
    OpenDocs(String),
    /// Open an arbitrary URL that the shell has already vouched for.
    OpenUrl(String),
    /// Open the directory apps are installed into.
    OpenInstallRoot,
    /// Inspect a repository without adding it.
    CheckRepository(String),
    /// Add a repository to the user's catalog, and point it at a folder on this machine.
    ///
    /// Named rather than positional: the two are both strings and mean completely different
    /// things, and `AddRepository(slug, folder)` would be an easy pair to transpose.
    AddRepository {
        /// `owner/name`, as read from the field.
        repo: String,
        /// Where the app already lives on this machine; empty when none was named.
        local: String,
    },
    /// Dismiss one notice.
    DismissNotice(usize),
    /// Dismiss every notice.
    ClearNotices,
    /// Start over in the add-repository flow.
    ResetProbe,
}

/// Why an action cannot be carried out yet, if it cannot.
///
/// Installation and removal are deliberately absent from this milestone: the
/// release conventions have to be settled first, and the interface says so rather
/// than offering a button that would fail. See `docs/RELEASES.md`.
#[must_use]
pub fn deferred_reason(action: &str) -> &'static str {
    match action {
        "install" | "update" => {
            "Installing and updating arrive once every repository publishes releases the \
             same way. See docs/RELEASES.md for the convention being adopted."
        }
        "uninstall" => {
            "Removal arrives with installation, so that the launcher never removes \
             something it cannot put back."
        }
        _ => "This is not available yet.",
    }
}

/// How an app's verdict should read.
#[must_use]
pub fn describe(status: &AppStatus) -> (&'static str, Tone) {
    match status {
        AppStatus::Unknown => ("Not checked", Tone::Neutral),
        AppStatus::Checking => ("Checking…", Tone::Info),
        AppStatus::Link => ("Link", Tone::Neutral),
        AppStatus::NoReleases => ("No releases yet", Tone::Warning),
        AppStatus::NoBuildForThisSystem { .. } => ("No build for this system", Tone::Warning),
        AppStatus::AmbiguousBuild { .. } => ("Several builds match", Tone::Warning),
        AppStatus::CheckFailed { .. } => ("Check failed", Tone::Danger),
        AppStatus::Available { .. } => ("Available", Tone::Accent),
        AppStatus::UpToDate => ("Up to date", Tone::Success),
        AppStatus::UpdateAvailable { .. } => ("Update available", Tone::Accent),
        AppStatus::Found { .. } => ("Found on this machine", Tone::Info),
        AppStatus::InstalledUncomparable { .. } => ("Installed, version unclear", Tone::Warning),
    }
}

/// The longer sentence under an app's badge.
#[must_use]
pub fn explain(card: &AppCard) -> String {
    match &card.status {
        AppStatus::Unknown => format!("{} has not been checked against GitHub yet.", card.entry.repo),
        AppStatus::Checking => "Asking GitHub what is available…".to_owned(),
        AppStatus::Link => "This entry opens a page rather than installing anything.".to_owned(),
        AppStatus::NoReleases => format!(
            "{} publishes no releases yet, so there is nothing to install. \
             The launcher will notice as soon as it does.",
            card.entry.repo
        ),
        AppStatus::NoBuildForThisSystem { tag } => format!(
            "Release {tag} has no build for {}. It is published for other systems only.",
            card.release
                .as_ref()
                .map_or_else(|| "this system".to_owned(), |_| "this system".to_owned())
        ),
        AppStatus::AmbiguousBuild { candidates, .. } => format!(
            "{} attached files match this system equally well ({}), so the launcher will not \
             guess. Add a `stems` or `extensions` entry to the catalog to choose.",
            candidates.len(),
            candidates.join(", ")
        ),
        AppStatus::CheckFailed { reason } => format!("GitHub could not be read: {reason}"),
        AppStatus::Available { version, .. } => {
            format!("Version {version} is published and ready.")
        }
        AppStatus::UpToDate => "The newest release is already the one you have.".to_owned(),
        AppStatus::UpdateAvailable {
            installed, latest, ..
        } => format!("{installed} is installed; {latest} has been published."),
        AppStatus::Found { .. } => match &card.detection_path {
            Some(path) => {
                format!("Found at {path}. The launcher did not install it and will not update or remove it.")
            }
            None => "Found on this machine, but not installed by the launcher.".to_owned(),
        },
        AppStatus::InstalledUncomparable {
            installed, latest, ..
        } => format!(
            "Installed as `{installed}`, which is not a version this launcher can compare with \
             `{latest}`. It will not claim an update either way."
        ),
    }
}
