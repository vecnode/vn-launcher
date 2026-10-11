//! The desktop shell: it owns the world and carries out what the interface asks for.
//!
//! This is the only file in the workspace that knows about all three crates at once.
//! The interface hands it a list of [`Action`]s and it decides what those mean —
//! read the catalog, ask the worker, start a process, open a page, write a file.
//! Keeping that decision here is what lets the interface be tested against a
//! literal `World` and the pure core be tested against literals of its own.

use eframe::Storage;
use vnl_core::catalog::{AppEntry, Catalog};
use vnl_core::status::{self, AppStatus, Availability, InstallState};
use vnl_core::{Os, Repo, Target};
use vnl_native::{Paths, launch, user_catalog};
use vnl_ui::{Action, AppCard, LauncherUpdate, Notice, Tab, Tone, World};

use crate::settings::Settings;
use crate::worker::{Request, Response, Worker, now_unix};

/// The application.
#[derive(Debug)]
pub(crate) struct Shell {
    ui: vnl_ui::Ui,
    world: World,
    worker: Worker,
    paths: Paths,
    catalog: Catalog,
    settings: Settings,
    /// Problems found while starting up, shown once the window exists.
    startup_notices: Vec<Notice>,
    /// Whether the background reader has already been reported as gone, so the
    /// notice is shown once rather than every frame.
    worker_dead: bool,
}

impl Shell {
    /// Build the application and start the first check.
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Before anything is drawn. `Context::set_fonts` only takes effect at the
        // start of the next pass, and the launcher's own named families — medium,
        // semibold, bold — are used by the very first frame's text.
        vnl_ui::fonts::install(&cc.egui_ctx);

        let settings = Settings::load(cc.storage);
        let mut startup_notices = Vec::new();

        let paths = match Paths::discover() {
            Ok(paths) => paths,
            Err(error) => {
                // Without a home directory there is nowhere to put a catalog or a
                // cache, but the launcher can still show what it ships with.
                startup_notices.push(Notice::error(
                    "The launcher has nowhere to keep its files",
                    error.to_string(),
                ));
                Paths::under(std::env::temp_dir())
            }
        };

        let catalog = load_catalog(&paths, &mut startup_notices);
        let target = Target::current();
        let mut world = build_world(&catalog, &paths, target, &settings);
        world.notices = startup_notices.clone();

        let worker = match Worker::start(paths.clone(), target, cc.egui_ctx.clone()) {
            Ok(worker) => worker,
            Err(error) => {
                startup_notices.push(Notice::error(
                    "The background reader could not be started",
                    format!(
                        "{error}. The window will open, but nothing will be checked against \
                         GitHub."
                    ),
                ));
                Worker::idle()
            }
        };
        worker.send(Request::SetPrerelease(settings.prerelease));
        worker.send(Request::SetOffline(settings.offline));
        worker.send(Request::Check(checkable_entries(&catalog)));
        world.busy = true;

        let ui = vnl_ui::Ui::new(settings.theme, settings.prerelease);

        Self {
            ui,
            world,
            worker,
            paths,
            catalog,
            settings,
            startup_notices,
            worker_dead: false,
        }
    }

    /// Take in whatever the worker has finished since the last frame.
    fn pump(&mut self) {
        let (responses, alive) = self.worker.drain();

        for response in responses {
            match response {
                Response::App {
                    id,
                    availability,
                    detection,
                    checked_at,
                } => self.apply_app(id, availability, detection, checked_at),
                Response::Probe(report) => {
                    self.ui.probing = false;
                    self.ui.probe = Some(*report);
                }
                Response::LauncherUpdate(update) => {
                    self.world.update = *update;
                }
                Response::CacheCleared { remaining } => {
                    self.world.cache_entries = remaining;
                    self.notice(Notice::success("Cached release feeds forgotten"));
                }
                Response::Finished { cached } => {
                    self.world.busy = false;
                    self.world.checked_at = Some(now_unix());
                    self.world.cache_entries = cached;
                }
            }
        }

        // A worker that has gone away would otherwise leave every card saying
        // "Checking…" for the rest of the session.
        if !alive && !self.worker_dead {
            self.worker_dead = true;
            self.world.busy = false;
            self.notice(Notice::error(
                "The background reader stopped",
                "Nothing further will be checked. Restart the launcher to try again.",
            ));
        }
    }

    /// Fold one app's answer into the world.
    fn apply_app(
        &mut self,
        id: String,
        availability: Availability,
        detection: vnl_native::Detection,
        checked_at: i64,
    ) {
        let Some(card) = self.world.apps.iter_mut().find(|card| card.id() == id) else {
            return;
        };
        card.install = detection.to_install_state();
        card.detection_path = detection.path().map(|path| path.display().to_string());
        card.detection_checked = detection.checked.clone();
        card.release = match &availability {
            Availability::Release(info) => Some((**info).clone()),
            _ => None,
        };
        card.status = status::compute(&card.install, &availability, card.entry.kind);
        card.checked_at = Some(checked_at);
    }

    /// Add a message, keeping only the most recent handful.
    fn notice(&mut self, notice: Notice) {
        self.world.notices.push(notice);
        while self.world.notices.len() > 4 {
            self.world.notices.remove(0);
        }
    }

    /// Carry out one thing the person asked for.
    fn handle(&mut self, action: Action) {
        match action {
            Action::Refresh => {
                self.check_all();
            }
            Action::RefreshApp(id) => {
                if let Some(entry) = self.catalog.get(&id) {
                    self.world.busy = true;
                    self.worker.send(Request::CheckOne(Box::new(entry.clone())));
                }
            }
            Action::Select(id) => {
                self.ui.selected = Some(id);
                self.ui.tab = Tab::Library;
            }
            Action::Back => self.ui.selected = None,
            Action::SetTab(tab) => {
                if tab != Tab::Library {
                    self.ui.selected = None;
                }
                self.ui.tab = tab;
            }
            Action::SetSearch(text) => self.ui.search = text,
            Action::SetTheme(choice) => {
                self.ui.theme_choice = choice;
                self.settings.theme = choice;
            }
            Action::SetPrerelease(value) => {
                self.ui.prerelease = value;
                self.settings.prerelease = value;
                self.worker.send(Request::SetPrerelease(value));
                self.check_all();
            }
            Action::CheckLauncherUpdate => {
                self.world.update = LauncherUpdate::Checking;
                self.worker.send(Request::CheckLauncherUpdate);
            }
            Action::SetOffline(value) => {
                self.world.offline = value;
                self.settings.offline = value;
                self.worker.send(Request::SetOffline(value));
                if !value {
                    self.check_all();
                }
            }
            Action::ClearCache => self.worker.send(Request::ClearCache),
            Action::Launch(id) => self.launch(&id),
            Action::OpenRepository(id) => {
                let url = self.world.app(&id).map(|card| card.entry.repo.html_url());
                self.open(url, "the repository");
            }
            Action::OpenHomepage(id) => {
                let url = self.world.app(&id).map(|card| card.entry.homepage_or_repo());
                self.open(url, "the home page");
            }
            Action::OpenDocs(id) => {
                let url = self.world.app(&id).map(|card| card.entry.install_docs_url());
                self.open(url, "the install instructions");
            }
            Action::OpenUrl(url) => self.open(Some(url), "the page"),
            Action::OpenInstallRoot => self.open_install_root(),
            Action::CheckRepository(text) => {
                self.ui.probing = true;
                self.ui.probe = None;
                self.worker.send(Request::Probe(text));
            }
            Action::AddRepository { repo, local } => self.add_repository(&repo, &local),
            Action::ResetProbe => {
                self.ui.probe = None;
                self.ui.probing = false;
                self.ui.add_input.clear();
                self.ui.local_input.clear();
            }
            Action::DismissNotice(index) => {
                if index < self.world.notices.len() {
                    self.world.notices.remove(index);
                }
            }
            Action::ClearNotices => self.world.notices.clear(),
        }
    }

    /// Ask the worker about every app that has something to look up.
    fn check_all(&mut self) {
        let entries = checkable_entries(&self.catalog);
        if entries.is_empty() {
            return;
        }
        self.world.busy = true;
        for card in &mut self.world.apps {
            if matches!(card.status, AppStatus::Link) {
                continue;
            }
            if !card.status.is_busy() {
                card.status = AppStatus::Checking;
            }
        }
        self.worker.send(Request::Check(entries));
    }

    /// Start an app the launcher can find.
    fn launch(&mut self, id: &str) {
        let Some(card) = self.world.app(id).cloned() else {
            return;
        };
        let os = self.world.target.map_or(Os::Linux, |target| target.os);
        let argv = card.entry.launch.for_os(os);

        if argv.is_empty() {
            self.notice(Notice {
                tone: Tone::Warning,
                title: format!("{} has no launch command", card.entry.name),
                detail: Some("It installs into DeepSeek Harness rather than running on its own.".to_owned()),
            });
            return;
        }

        let install_directory = self.paths.app_dir(id);
        let binary = card.detection_path.clone().unwrap_or_else(|| {
            let mut name = card.entry.install.binary_or(id);
            if os == Os::Windows && !name.to_ascii_lowercase().ends_with(".exe") {
                name.push_str(".exe");
            }
            install_directory.join(name).display().to_string()
        });

        let install = install_directory.display().to_string();
        let resolved: Vec<String> = argv
            .iter()
            .map(|argument| argument.replace("{install}", &install).replace("{bin}", &binary))
            .collect();

        match launch::spawn(&resolved, Some(&install_directory)) {
            Ok(pid) => log::info!("started {} as process {pid}", card.entry.name),
            Err(error) => self.notice(Notice::error(
                format!("Could not start {}", card.entry.name),
                error.to_string(),
            )),
        }
    }

    /// Open a URL the launcher is willing to vouch for.
    fn open(&mut self, url: Option<String>, what: &str) {
        let Some(url) = url else { return };
        if let Err(error) = launch::open_url(&url) {
            self.notice(Notice::error(format!("Could not open {what}"), error.to_string()));
        }
    }

    /// Reveal where apps will be installed.
    fn open_install_root(&mut self) {
        let root = self.paths.install_root();
        if let Err(error) = std::fs::create_dir_all(&root) {
            self.notice(Notice::error(
                "Could not create the install folder",
                error.to_string(),
            ));
            return;
        }
        if let Err(error) = launch::open_path(&root) {
            self.notice(Notice::error(
                "Could not open the install folder",
                error.to_string(),
            ));
        }
    }

    /// Write a repository into the user's catalog and track it.
    ///
    /// `local` is the second half of the flow: the repository says where the code comes
    /// from, this says where it already is. It is optional, and is stored as text rather
    /// than validated — the launcher does not need the folder to exist to remember it, and
    /// refusing a path because a drive is not mounted right now would be worse than
    /// keeping it.
    fn add_repository(&mut self, slug: &str, local: &str) {
        let repo = match Repo::parse(slug) {
            Ok(repo) => repo,
            Err(error) => {
                self.notice(Notice::error(
                    "That is not a repository the launcher can read",
                    error.to_string(),
                ));
                return;
            }
        };

        let folder = Some(local).filter(|path| !path.trim().is_empty());
        let path = self.paths.user_catalog();
        match user_catalog::append(&path, &repo, folder) {
            Ok(id) => {
                self.reload_catalog();
                self.ui.probe = None;
                self.ui.probing = false;
                self.ui.add_input.clear();
                self.ui.local_input.clear();
                let detail = match folder {
                    Some(folder) => format!("Written to {}, pointed at {folder}.", path.display()),
                    None => format!(
                        "Written to {}. Every field the launcher guesses can be overridden there.",
                        path.display()
                    ),
                };
                self.notice(Notice::success(format!("Added {repo}")).with_detail(detail));
                self.check_all();
                let _ = id;
            }
            Err(error) => self.notice(Notice::error("Could not add the repository", error.to_string())),
        }
    }

    /// Re-read the catalog and rebuild the app list, keeping what is already known.
    fn reload_catalog(&mut self) {
        let mut problems = Vec::new();
        self.catalog = load_catalog(&self.paths, &mut problems);
        for notice in problems {
            self.notice(notice);
        }
        self.world.user_catalog_exists = self.paths.user_catalog().exists();

        // Keep the answers already gathered for apps that are still present; a new
        // app starts out unchecked rather than pretending to know something.
        let previous = std::mem::take(&mut self.world.apps);
        self.world.apps = self
            .catalog
            .entries()
            .iter()
            .map(|entry| match previous.iter().find(|card| card.id() == entry.id) {
                Some(existing) => AppCard {
                    entry: entry.clone(),
                    ..existing.clone()
                },
                None => new_card(entry.clone()),
            })
            .collect();
    }

    /// Keep the settings file in step with what the interface chose.
    fn sync_settings(&mut self) {
        self.settings.theme = self.ui.theme_choice;
        self.settings.prerelease = self.ui.prerelease;
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        self.sync_settings();

        for action in self.ui.show(ui, &self.world) {
            self.handle(action);
        }

        // The worker answers at unpredictable moments; ask for a frame soon enough
        // that a card does not sit on "Checking…" until the next input event.
        if self.world.busy {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(120));
        }

        self.startup_notices.clear();
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.settings.save(storage);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
}

/// Read the built-in catalog and merge the user's on top of it.
fn load_catalog(paths: &Paths, problems: &mut Vec<Notice>) -> Catalog {
    let mut catalog = match Catalog::builtin() {
        Ok(catalog) => catalog,
        Err(error) => {
            problems.push(Notice::error(
                "The catalog this build ships with could not be read",
                error.to_string(),
            ));
            Catalog::default()
        }
    };

    let path = paths.user_catalog();
    match user_catalog::load(&path) {
        Ok(Some(overlay)) => {
            if let Err(error) = catalog.merge(overlay) {
                problems.push(Notice::error(
                    "Your catalog could not be merged",
                    error.to_string(),
                ));
            }
        }
        Ok(None) => {}
        Err(error) => problems.push(Notice::error(
            format!("{} could not be read", path.display()),
            format!("{error}. The launcher is using only the catalog it ships with."),
        )),
    }

    catalog
}

/// The entries worth asking GitHub about.
fn checkable_entries(catalog: &Catalog) -> Vec<AppEntry> {
    catalog
        .entries()
        .iter()
        .filter(|entry| entry.kind != vnl_core::AppKind::Link)
        .cloned()
        .collect()
}

/// Build the initial world.
fn build_world(catalog: &Catalog, paths: &Paths, target: Target, settings: &Settings) -> World {
    World {
        apps: catalog.entries().iter().cloned().map(new_card).collect(),
        notices: Vec::new(),
        target: Some(target),
        busy: false,
        checked_at: None,
        offline: settings.offline,
        cache_entries: 0,
        install_root: paths.install_root().display().to_string(),
        user_catalog: paths.user_catalog().display().to_string(),
        user_catalog_exists: paths.user_catalog().exists(),
        update: LauncherUpdate::Unknown,
    }
}

/// A card for an app nothing is known about yet.
fn new_card(entry: AppEntry) -> AppCard {
    AppCard {
        entry,
        status: AppStatus::Unknown,
        install: InstallState::Absent,
        detection_path: None,
        detection_checked: Vec::new(),
        release: None,
        checked_at: None,
    }
}
