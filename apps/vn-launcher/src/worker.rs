//! The background worker.
//!
//! Reading four repositories means four HTTP requests, and on a slow connection that
//! is seconds of waiting that must not happen on the thread drawing the window. The
//! worker owns the client, the feed and the on-disk cache for the whole session and
//! reports each app as it finishes, so cards fill in one at a time rather than all
//! at once at the end.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;

use vnl_core::catalog::{AppEntry, AppKind};
use vnl_core::status::Availability;
use vnl_core::{Repo, Tag, Target};
use vnl_native::cache::DEFAULT_TTL;
use vnl_native::http::{Client, MAX_PROBE_BYTES};
use vnl_native::{Detection, Feed, FeedCache, Paths};
use vnl_ui::{LauncherUpdate, ProbeReport};

/// What the shell asks the worker to do.
///
/// The apps to check travel with the request rather than living in the worker, so
/// adding a repository takes effect immediately instead of at the next restart.
#[derive(Debug, Clone)]
pub enum Request {
    /// Check these apps.
    Check(Vec<AppEntry>),
    /// Check one app.
    CheckOne(Box<AppEntry>),
    /// Inspect a repository that is not in the catalog yet.
    Probe(String),
    /// Offer pre-releases.
    SetPrerelease(bool),
    /// Stop contacting GitHub.
    SetOffline(bool),
    /// Ask whether the launcher itself has a newer release.
    ///
    /// The only way that question is ever asked: there is no schedule and no
    /// background check.
    CheckLauncherUpdate,
    /// Forget every cached response.
    ClearCache,
}

/// What the worker reports back.
#[derive(Debug)]
pub enum Response {
    /// One app's answer.
    App {
        /// Which app.
        id: String,
        /// What GitHub offers.
        availability: Availability,
        /// What is on this machine.
        detection: Detection,
        /// When this was established.
        checked_at: i64,
    },
    /// What the add-repository flow learned.
    Probe(Box<ProbeReport>),
    /// What the launcher's own repository offers.
    LauncherUpdate(Box<LauncherUpdate>),
    /// The cache was emptied.
    CacheCleared {
        /// The cache's state afterwards.
        remaining: usize,
    },
    /// A batch of work finished.
    Finished {
        /// How many responses the cache now holds, so the shell can report the real
        /// number rather than guessing at one.
        cached: usize,
    },
}

/// The handle the shell keeps.
#[derive(Debug)]
pub struct Worker {
    requests: Sender<Request>,
    responses: Receiver<Response>,
}

impl Worker {
    /// Start the worker. It runs until the process ends.
    ///
    /// # Errors
    /// Returns the operating system's error when the thread cannot be created, which
    /// happens only under a severe resource shortage. The caller carries on with
    /// [`Worker::idle`] rather than failing to start.
    pub fn start(paths: Paths, target: Target, ctx: egui::Context) -> std::io::Result<Self> {
        let (request_tx, request_rx) = channel();
        let (response_tx, response_rx) = channel();
        thread::Builder::new()
            .name("vnl-feed".to_owned())
            .spawn(move || run(&paths, target, &ctx, &request_rx, &response_tx))?;
        Ok(Self {
            requests: request_tx,
            responses: response_rx,
        })
    }

    /// A worker that answers nothing.
    ///
    /// Used when the thread could not be started, so the window still opens and can
    /// still show the catalog — it simply never learns anything new, and the notice
    /// explains why.
    #[must_use]
    pub fn idle() -> Self {
        // The request receiver is dropped immediately, so every send fails and every
        // drain is empty: exactly the behaviour wanted, with no special-casing.
        let (request_tx, _) = channel();
        let (_, response_rx) = channel();
        Self {
            requests: request_tx,
            responses: response_rx,
        }
    }

    /// Ask for something. A dead worker is not an error worth surfacing: the
    /// interface simply stops receiving answers.
    pub fn send(&self, request: Request) {
        if self.requests.send(request).is_err() {
            log::warn!("the feed worker is no longer running");
        }
    }

    /// Take whatever the worker has said since the last call.
    ///
    /// The flag is `false` when the worker's channel has closed, which is how the
    /// shell learns that the thread died. Without it, a panic anywhere inside a
    /// library the worker calls would leave the window saying "checking…" for ever.
    pub fn drain(&self) -> (Vec<Response>, bool) {
        let mut responses = Vec::new();
        loop {
            match self.responses.try_recv() {
                Ok(response) => responses.push(response),
                Err(TryRecvError::Empty) => return (responses, true),
                Err(TryRecvError::Disconnected) => return (responses, false),
            }
        }
    }
}

/// The worker's whole life.
fn run(
    paths: &Paths,
    target: Target,
    ctx: &egui::Context,
    requests: &Receiver<Request>,
    responses: &Sender<Response>,
) {
    if let Err(error) = paths.ensure_dirs() {
        log::warn!("could not create the launcher's directories: {error}");
    }

    let cache = FeedCache::load(paths.feed_cache(), DEFAULT_TTL);
    let mut feed = Feed::new(Client::new(), cache);
    let client = Client::new();

    while let Ok(request) = requests.recv() {
        match request {
            Request::Check(entries) => {
                for entry in &entries {
                    check(&mut feed, entry, target, paths, responses);
                }
                let _ = responses.send(Response::Finished {
                    cached: feed.cache().len(),
                });
            }
            Request::CheckOne(entry) => {
                check(&mut feed, &entry, target, paths, responses);
                let _ = responses.send(Response::Finished {
                    cached: feed.cache().len(),
                });
            }
            Request::Probe(text) => {
                let report = probe(&mut feed, &client, &text, target);
                let _ = responses.send(Response::Probe(Box::new(report)));
                let _ = responses.send(Response::Finished {
                    cached: feed.cache().len(),
                });
            }
            Request::SetPrerelease(value) => feed.set_allow_prerelease(value),
            Request::SetOffline(value) => feed.set_offline(value),
            Request::CheckLauncherUpdate => {
                let update = launcher_update(&mut feed, target);
                let _ = responses.send(Response::LauncherUpdate(Box::new(update)));
                let _ = responses.send(Response::Finished {
                    cached: feed.cache().len(),
                });
            }
            Request::ClearCache => {
                if let Err(error) = feed.cache_mut().clear() {
                    log::warn!("could not clear the feed cache: {error}");
                }
                let _ = responses.send(Response::CacheCleared {
                    remaining: feed.cache().len(),
                });
            }
        }

        if let Err(error) = feed.cache_mut().save() {
            log::warn!("could not save the feed cache: {error}");
        }
        // Wake the window so the answer is drawn.
        ctx.request_repaint();
    }
}

/// Check one app and report the answer.
fn check(feed: &mut Feed, entry: &AppEntry, target: Target, paths: &Paths, responses: &Sender<Response>) {
    let detection = vnl_native::detect::detect(&entry.detect, &paths.app_dir(&entry.id));
    let availability = if entry.kind == AppKind::Link {
        // A link entry has nothing to look up; asking GitHub about it would spend a
        // request to learn something the catalog already says.
        Availability::Unknown
    } else {
        feed.availability(entry, &target)
    };

    let _ = responses.send(Response::App {
        id: entry.id.clone(),
        availability,
        detection,
        checked_at: now_unix(),
    });
}

/// Inspect a repository for the add-repository flow.
///
/// Existence is checked against the repository endpoint first, because the releases
/// endpoint answers `404` for both "no such repository" and "no releases" — and
/// those two need very different sentences.
fn probe(feed: &mut Feed, client: &Client, text: &str, target: Target) -> ProbeReport {
    let repo = match Repo::parse(text) {
        Ok(repo) => repo,
        Err(error) => {
            return ProbeReport {
                repo: text.trim().to_owned(),
                ok: false,
                summary: error.to_string(),
                release_count: 0,
                recent_tags: Vec::new(),
                matched_asset: None,
            };
        }
    };

    match client.get_json(&repo.api_repo_url(), MAX_PROBE_BYTES) {
        Ok((200, _)) => {}
        Ok((404, _)) => {
            return ProbeReport {
                repo: repo.slug(),
                ok: false,
                summary: format!(
                    "{repo} does not exist, or is private. The launcher can only read public \
                     repositories."
                ),
                release_count: 0,
                recent_tags: Vec::new(),
                matched_asset: None,
            };
        }
        Ok((status, _)) => {
            return ProbeReport {
                repo: repo.slug(),
                ok: false,
                summary: format!("GitHub returned HTTP {status} for {repo}."),
                release_count: 0,
                recent_tags: Vec::new(),
                matched_asset: None,
            };
        }
        Err(error) => {
            return ProbeReport {
                repo: repo.slug(),
                ok: false,
                summary: format!("Could not reach GitHub about {repo}: {error}"),
                release_count: 0,
                recent_tags: Vec::new(),
                matched_asset: None,
            };
        }
    }

    // With no entry to go on, the asset stem is assumed to be the repository name.
    let stems = vec![repo.name().to_ascii_lowercase()];
    let probe = feed.probe(&repo, &target, &stems, &[]);
    let matched = probe
        .latest
        .as_ref()
        .and_then(|info| info.asset.chosen().map(|asset| asset.name.clone()));

    let summary = match &probe.availability {
        Availability::NoReleases => format!(
            "{repo} publishes no releases yet. It can still be added and tracked, and the \
             launcher will notice as soon as it publishes one."
        ),
        Availability::Failed(reason) => format!("Could not read {repo}'s releases: {reason}"),
        Availability::Release(info) if matched.is_some() => format!(
            "{} release(s). The newest is {}, and {} fits this machine.",
            probe.release_count,
            info.tag,
            matched.as_deref().unwrap_or("nothing")
        ),
        Availability::Release(info) => format!(
            "{} release(s). The newest is {}, but none of its {} attached file(s) fits this \
             machine — it is published for other systems.",
            probe.release_count, info.tag, info.asset_count
        ),
        Availability::Unknown | Availability::Checking => {
            format!("{} release(s) found.", probe.release_count)
        }
    };

    ProbeReport {
        repo: repo.slug(),
        ok: true,
        summary,
        release_count: probe.release_count,
        recent_tags: probe.recent_tags,
        matched_asset: matched,
    }
}

/// Ask the launcher's own repository whether a newer release exists.
///
/// Run through the same feed as every other app, deliberately: a second code path for
/// "check myself" would be a second place for the answer to be wrong.
fn launcher_update(feed: &mut Feed, target: Target) -> LauncherUpdate {
    let repo = match Repo::parse(vnl_core::LAUNCHER_REPO) {
        Ok(repo) => repo,
        Err(error) => return LauncherUpdate::Failed(error.to_string()),
    };

    // The stems only matter for choosing an asset, which an update check does not do.
    let probe = feed.probe(&repo, &target, &["vn-launcher".to_owned()], &[]);
    decide_launcher_update(&probe.availability, vnl_core::VERSION)
}

/// Turn what the launcher's own repository offers into an answer for this build.
///
/// Pure, and separate from the request that produced it, so the interesting cases —
/// no releases yet, a tag that is not a version, being current — can be tested without
/// a network or an interface.
fn decide_launcher_update(availability: &Availability, running: &str) -> LauncherUpdate {
    let Availability::Release(info) = availability else {
        return match availability {
            Availability::Failed(reason) => LauncherUpdate::Failed(reason.clone()),
            // Not asked yet, still being asked, or nothing published. None of these is
            // an error, and only one of them is worth a sentence of its own.
            Availability::Unknown | Availability::Checking => LauncherUpdate::Unknown,
            Availability::NoReleases | Availability::Release(_) => LauncherUpdate::NoReleases,
        };
    };

    match vnl_core::version::is_newer(&Tag::parse(&info.tag), &Tag::parse(running)) {
        Some(true) => LauncherUpdate::Available {
            version: info.version.clone(),
            url: info.html_url.clone(),
        },
        Some(false) => LauncherUpdate::UpToDate,
        // A tag that is not a version cannot be compared, and guessing would be worse
        // than saying so.
        None => LauncherUpdate::Failed(format!(
            "the newest release is tagged `{}`, which cannot be compared with {running}",
            info.tag
        )),
    }
}

/// Seconds since the Unix epoch.
#[must_use]
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use vnl_core::status::ReleaseInfo;
    use vnl_core::{Asset, AssetChoice};

    fn release(tag: &str, version: &str) -> Availability {
        Availability::Release(Box::new(ReleaseInfo {
            tag: tag.to_owned(),
            version: version.to_owned(),
            name: None,
            prerelease: false,
            published_at: None,
            html_url: format!("https://github.com/vecnode/vn-launcher/releases/tag/{tag}"),
            notes: None,
            asset: AssetChoice::Chosen(Asset {
                name: format!("vn-launcher-{version}-windows-x86_64.zip"),
                size: 1,
                download_url: String::new(),
                digest: None,
            }),
            asset_count: 1,
        }))
    }

    #[test]
    fn being_current_is_reported_as_current() {
        assert_eq!(
            decide_launcher_update(&release("v0.1.0", "0.1.0"), "0.1.0"),
            LauncherUpdate::UpToDate
        );
        // An older release than the running build is also "nothing to do".
        assert_eq!(
            decide_launcher_update(&release("v0.0.9", "0.0.9"), "0.1.0"),
            LauncherUpdate::UpToDate
        );
    }

    #[test]
    fn a_newer_release_is_offered_with_its_page() {
        match decide_launcher_update(&release("v0.2.0", "0.2.0"), "0.1.0") {
            LauncherUpdate::Available { version, url } => {
                assert_eq!(version, "0.2.0");
                assert!(url.contains("v0.2.0"));
            }
            other => panic!("expected Available, got {other:?}"),
        }
    }

    #[test]
    fn a_repository_with_nothing_published_is_not_an_error() {
        // The real state of `vecnode/vn-launcher` today, and the answer a person
        // pressing Update will actually see.
        assert_eq!(
            decide_launcher_update(&Availability::NoReleases, "0.1.0"),
            LauncherUpdate::NoReleases
        );
        assert_eq!(
            decide_launcher_update(&Availability::Unknown, "0.1.0"),
            LauncherUpdate::Unknown
        );
    }

    #[test]
    fn an_unreadable_feed_is_reported_rather_than_hidden() {
        assert_eq!(
            decide_launcher_update(&Availability::Failed("offline".to_owned()), "0.1.0"),
            LauncherUpdate::Failed("offline".to_owned())
        );
    }

    #[test]
    fn a_tag_that_is_not_a_version_is_never_guessed_at() {
        match decide_launcher_update(&release("nightly", "nightly"), "0.1.0") {
            LauncherUpdate::Failed(reason) => {
                assert!(reason.contains("nightly"), "{reason}");
                assert!(reason.contains("0.1.0"), "{reason}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }
}
