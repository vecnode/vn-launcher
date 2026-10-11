//! Reading release feeds, through the cache.
//!
//! The feed's contract is that it never fails: it answers with an
//! [`Availability`], and "the network is down" or "GitHub is unhappy" are answers
//! like any other. That keeps every caller from having to invent its own idea of
//! what an unreadable feed means.

use vnl_core::catalog::AppEntry;
use vnl_core::release;
use vnl_core::status::{Availability, ReleaseInfo};
use vnl_core::{Repo, Target};

use crate::cache::FeedCache;
use crate::http::{self, Client, MAX_FEED_BYTES};

/// How many releases to ask for in one request.
const PER_PAGE: usize = 20;

/// How many recent tags the add-repository preview shows.
const RECENT_TAGS: usize = 5;

/// What a repository that is not yet in the catalog offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The repository that was inspected.
    pub repo: Repo,
    /// What the feed concluded.
    pub availability: Availability,
    /// How many releases are published.
    pub release_count: usize,
    /// The newest offerable release, summarised for this machine.
    pub latest: Option<ReleaseInfo>,
    /// The most recent tags, newest first.
    pub recent_tags: Vec<String>,
}

/// Reads release feeds and remembers them.
#[derive(Debug)]
pub struct Feed {
    client: Client,
    cache: FeedCache,
    allow_prerelease: bool,
    offline: bool,
}

impl Feed {
    /// A feed reading through `cache` with `client`.
    #[must_use]
    pub fn new(client: Client, cache: FeedCache) -> Self {
        Self {
            client,
            cache,
            allow_prerelease: false,
            offline: false,
        }
    }

    /// Whether pre-releases are offered.
    #[must_use]
    pub fn allow_prerelease(&self) -> bool {
        self.allow_prerelease
    }

    /// Whether to offer pre-releases.
    pub fn set_allow_prerelease(&mut self, value: bool) {
        self.allow_prerelease = value;
    }

    /// Whether network reads are suppressed.
    #[must_use]
    pub fn is_offline(&self) -> bool {
        self.offline
    }

    /// Suppress network reads, serving only what is cached.
    pub fn set_offline(&mut self, value: bool) {
        self.offline = value;
    }

    /// The cache in use.
    #[must_use]
    pub fn cache(&self) -> &FeedCache {
        &self.cache
    }

    /// The cache in use, mutably, so a caller can change its lifetime or save it.
    pub fn cache_mut(&mut self) -> &mut FeedCache {
        &mut self.cache
    }

    /// What GitHub offers for a catalog entry.
    pub fn availability(&mut self, entry: &AppEntry, target: &Target) -> Availability {
        let url = releases_url(&entry.repo);
        match self.body(&url) {
            Err(availability) => availability,
            Ok(body) => interpret(
                &body,
                target,
                &entry.install.stems,
                &entry.install.extensions,
                self.allow_prerelease,
            ),
        }
    }

    /// Inspect a repository that is not in the catalog yet.
    ///
    /// Used by the add-repository flow so a person can see what they are about to
    /// add â€” how many releases there are, and which attached file would be chosen
    /// for *this* machine â€” before committing to it.
    pub fn probe(&mut self, repo: &Repo, target: &Target, stems: &[String], extensions: &[String]) -> Probe {
        let mut probe = Probe {
            repo: repo.clone(),
            availability: Availability::Unknown,
            release_count: 0,
            latest: None,
            recent_tags: Vec::new(),
        };

        let url = releases_url(repo);
        let body = match self.body(&url) {
            Err(availability) => {
                probe.availability = availability;
                return probe;
            }
            Ok(body) => body,
        };

        let releases = match release::parse(&body) {
            Err(error) => {
                probe.availability = Availability::Failed(error.to_string());
                return probe;
            }
            Ok(releases) => releases,
        };

        if releases.is_empty() {
            probe.availability = Availability::NoReleases;
            return probe;
        }

        probe.release_count = releases.len();
        probe.recent_tags = releases
            .iter()
            .filter(|release| !release.draft)
            .take(RECENT_TAGS)
            .map(|release| release.tag.raw().to_owned())
            .collect();

        match release::select_latest(&releases, self.allow_prerelease) {
            Some(latest) => {
                let info = ReleaseInfo::evaluate(latest, target, stems, extensions);
                probe.availability = Availability::Release(Box::new(info.clone()));
                probe.latest = Some(info);
            }
            None => probe.availability = Availability::NoReleases,
        }

        probe
    }

    /// The body for a URL, from the cache when it is fresh.
    ///
    /// The error type is [`Availability`] rather than [`crate::Error`] because every
    /// failure here has a defined meaning to the interface.
    fn body(&mut self, url: &str) -> core::result::Result<String, Availability> {
        if self.offline {
            return Err(Availability::Failed(
                "offline mode is on, so GitHub was not consulted".to_owned(),
            ));
        }

        if let Some(cached) = self.cache.get(url) {
            log::debug!("feed cache hit for {url}");
            return Ok(cached.to_owned());
        }

        match self.client.get_json(url, MAX_FEED_BYTES) {
            Ok((200, body)) => {
                self.cache.put(url, &body);
                Ok(body)
            }
            // A repository with no releases is not an error; it is the answer.
            Ok((404 | 410, _)) => Err(Availability::NoReleases),
            Ok((status, body)) => {
                let message = http::api_message(&body).unwrap_or_default();
                Err(Availability::Failed(if message.is_empty() {
                    format!("GitHub returned HTTP {status}")
                } else {
                    format!("GitHub returned HTTP {status}: {message}")
                }))
            }
            Err(error) => Err(Availability::Failed(error.to_string())),
        }
    }
}

/// The releases endpoint, asking for enough releases to be useful.
fn releases_url(repo: &Repo) -> String {
    format!("{}?per_page={PER_PAGE}", repo.api_releases_url())
}

/// Turn a release payload into the launcher's answer.
fn interpret(
    body: &str,
    target: &Target,
    stems: &[String],
    extensions: &[String],
    allow_prerelease: bool,
) -> Availability {
    let releases = match release::parse(body) {
        Ok(releases) => releases,
        Err(error) => return Availability::Failed(error.to_string()),
    };

    match release::select_latest(&releases, allow_prerelease) {
        Some(latest) => {
            Availability::Release(Box::new(ReleaseInfo::evaluate(latest, target, stems, extensions)))
        }
        None => Availability::NoReleases,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use vnl_core::platform::{Arch, Os};

    const VNCLI: &str = include_str!("../../core/tests/data/vncli-releases.json");

    fn windows() -> Target {
        Target {
            os: Os::Windows,
            arch: Arch::X86_64,
        }
    }

    fn stems() -> Vec<String> {
        vec!["vncli".to_owned()]
    }

    fn extensions() -> Vec<String> {
        vec!["zip".to_owned(), "tar.gz".to_owned()]
    }

    #[test]
    fn a_real_payload_becomes_the_windows_asset() {
        let availability = interpret(VNCLI, &windows(), &stems(), &extensions(), false);
        let Availability::Release(info) = availability else {
            panic!("expected a release");
        };
        assert_eq!(info.tag, "v0.1.0");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.asset_count, 2);
        assert_eq!(
            info.asset.chosen().map(|asset| asset.name.as_str()),
            Some("vncli-0.1.0-x86_64-pc-windows-msvc.zip")
        );
        assert!(!info.prerelease);
    }

    #[test]
    fn an_empty_release_list_is_not_an_error() {
        assert_eq!(
            interpret("[]", &windows(), &stems(), &extensions(), false),
            Availability::NoReleases
        );
    }

    #[test]
    fn a_malformed_payload_is_reported_not_hidden() {
        match interpret("<html>", &windows(), &stems(), &extensions(), false) {
            Availability::Failed(reason) => assert!(!reason.is_empty()),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn the_platform_is_honoured_when_choosing_the_asset() {
        let macos = Target {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        let availability = interpret(VNCLI, &macos, &stems(), &extensions(), false);
        let Availability::Release(info) = availability else {
            panic!("expected a release");
        };
        // The release exists; it simply has no macOS build.
        assert!(info.asset.chosen().is_none());
    }

    #[test]
    fn the_releases_url_asks_for_a_page_of_releases() {
        let repo = Repo::parse("vecnode/vncli").unwrap();
        assert_eq!(
            releases_url(&repo),
            "https://api.github.com/repos/vecnode/vncli/releases?per_page=20"
        );
    }
}
