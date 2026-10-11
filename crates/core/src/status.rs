//! The single verdict the interface renders for an app.
//!
//! Splitting "what is on this machine" ( [`InstallState`] ) from "what GitHub
//! offers" ( [`Availability`] ) from "what that means" ( [`AppStatus`] ) keeps the
//! decision in one pure function, [`compute`], which is where the interesting
//! cases are tested. The interface never re-derives a verdict.

use crate::asset::{AssetChoice, choose};
use crate::catalog::AppKind;
use crate::platform::Target;
use crate::release::Release;
use crate::version::{Tag, is_newer};

/// What the launcher knows about an app's presence on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InstallState {
    /// Nothing was found and no installation is recorded.
    #[default]
    Absent,
    /// Present on the machine, but installed by something other than this launcher.
    Found {
        /// Where it was found, when that is known.
        path: Option<String>,
    },
    /// Installed and tracked by this launcher.
    Managed {
        /// The version that was installed.
        version: String,
        /// Where it lives.
        path: Option<String>,
    },
}

impl InstallState {
    /// The version, when one is known.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Managed { version, .. } => Some(version),
            Self::Absent | Self::Found { .. } => None,
        }
    }

    /// Whether this launcher put it there.
    #[must_use]
    pub fn is_managed(&self) -> bool {
        matches!(self, Self::Managed { .. })
    }
}

/// What GitHub offers for an app.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Availability {
    /// The feed has not been consulted.
    #[default]
    Unknown,
    /// A request is in flight.
    Checking,
    /// The repository publishes no releases at all.
    NoReleases,
    /// The newest offerable release, summarised.
    Release(Box<ReleaseInfo>),
    /// The feed could not be read.
    Failed(String),
}

/// One release, reduced to what the interface needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// The raw tag.
    pub tag: String,
    /// The version to display.
    pub version: String,
    /// The release title, when it has one.
    pub name: Option<String>,
    /// Whether GitHub marks it a pre-release.
    pub prerelease: bool,
    /// When it was published.
    pub published_at: Option<String>,
    /// The release page.
    pub html_url: String,
    /// The notes.
    pub notes: Option<String>,
    /// Which attached file belongs on this machine.
    pub asset: AssetChoice,
    /// How many files are attached in total.
    pub asset_count: usize,
}

impl ReleaseInfo {
    /// Reduce a release to what matters for `target`.
    #[must_use]
    pub fn evaluate(release: &Release, target: &Target, stems: &[String], extensions: &[String]) -> Self {
        Self {
            tag: release.tag.raw().to_owned(),
            version: release.version(),
            name: release.name.clone(),
            prerelease: release.prerelease,
            published_at: release.published_at.clone(),
            html_url: release.html_url.clone(),
            notes: release.notes.clone(),
            asset: choose(&release.assets, target, stems, extensions),
            asset_count: release.assets.len(),
        }
    }
}

/// The verdict for one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    /// Nothing has been consulted yet.
    Unknown,
    /// A check is in flight.
    Checking,
    /// The entry is a link; there is nothing to install.
    Link,
    /// The repository publishes no releases.
    NoReleases,
    /// Releases exist, but none carries a build for this system.
    NoBuildForThisSystem {
        /// The tag that was examined.
        tag: String,
    },
    /// More than one asset fits this system equally well, so the launcher will not choose.
    AmbiguousBuild {
        /// The tag that was examined.
        tag: String,
        /// The names that tied.
        candidates: Vec<String>,
    },
    /// The feed could not be read.
    CheckFailed {
        /// Why, in words a person can act on.
        reason: String,
    },
    /// Not installed, with a build ready to install.
    Available {
        /// The version on offer.
        version: String,
        /// Its tag.
        tag: String,
    },
    /// Installed and current.
    UpToDate,
    /// Installed, and a newer release exists.
    UpdateAvailable {
        /// What is installed.
        installed: String,
        /// What is on offer.
        latest: String,
        /// The tag of the newer release.
        tag: String,
    },
    /// Present on the machine, but not installed by this launcher.
    Found {
        /// Its version, when the app reports one.
        version: Option<String>,
    },
    /// Installed, but the installed version and the release tag cannot be compared.
    InstalledUncomparable {
        /// What is recorded as installed.
        installed: String,
        /// What GitHub offers.
        latest: String,
        /// The tag of the offered release.
        tag: String,
    },
}

impl AppStatus {
    /// Whether a check is currently running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Checking)
    }

    /// Whether this app is worth drawing a person's eye to.
    ///
    /// [`Self::Available`] is deliberately absent. An app that has never been
    /// installed is not an update, and counting it would put a badge on the Updates
    /// tab for something that is not waiting on anyone — which, while installing is
    /// still switched off, would be a promise the launcher cannot keep.
    #[must_use]
    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            Self::UpdateAvailable { .. } | Self::AmbiguousBuild { .. } | Self::CheckFailed { .. }
        )
    }

    /// Whether an installation could be started from this state.
    ///
    /// Installation itself is not implemented yet — see `docs/RELEASES.md` — so the
    /// interface uses this to decide whether to explain why the action is absent.
    #[must_use]
    pub fn is_installable(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

/// Decide what to say about an app.
///
/// `kind` short-circuits link entries, which have nothing to install. Otherwise the
/// verdict is the combination of what is on the machine and what is on offer, and
/// the one case that must never be guessed at — an installed version that cannot be
/// compared with a tag — is reported as such rather than as an update.
#[must_use]
pub fn compute(install: &InstallState, availability: &Availability, kind: AppKind) -> AppStatus {
    if kind == AppKind::Link {
        return AppStatus::Link;
    }

    let release = match availability {
        Availability::Unknown => return AppStatus::Unknown,
        Availability::Checking => return AppStatus::Checking,
        Availability::NoReleases => return AppStatus::NoReleases,
        Availability::Failed(reason) => {
            return AppStatus::CheckFailed {
                reason: reason.clone(),
            };
        }
        Availability::Release(info) => info,
    };

    let tag = release.tag.clone();

    // Whether this machine can be served at all comes first: an app with no build
    // for this system is unavailable regardless of what is installed.
    match &release.asset {
        AssetChoice::None => return AppStatus::NoBuildForThisSystem { tag },
        AssetChoice::Ambiguous(candidates) => {
            return AppStatus::AmbiguousBuild {
                tag,
                candidates: candidates.clone(),
            };
        }
        AssetChoice::Chosen(_) => {}
    }

    match install {
        InstallState::Absent => AppStatus::Available {
            version: release.version.clone(),
            tag,
        },
        InstallState::Found { .. } => AppStatus::Found { version: None },
        InstallState::Managed { version, .. } => {
            match is_newer(&Tag::parse(&release.tag), &Tag::parse(version)) {
                Some(true) => AppStatus::UpdateAvailable {
                    installed: version.clone(),
                    latest: release.version.clone(),
                    tag,
                },
                Some(false) => AppStatus::UpToDate,
                None => AppStatus::InstalledUncomparable {
                    installed: version.clone(),
                    latest: release.version.clone(),
                    tag,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::asset::Asset;
    use crate::platform::{Arch, Os};

    fn windows_target() -> Target {
        Target {
            os: Os::Windows,
            arch: Arch::X86_64,
        }
    }

    fn release_with_asset(tag: &str) -> Release {
        Release {
            tag: Tag::parse(tag),
            name: None,
            notes: None,
            draft: false,
            prerelease: false,
            published_at: None,
            html_url: String::new(),
            assets: vec![Asset {
                name: format!("vncli-{tag}-x86_64-pc-windows-msvc.zip"),
                size: 10,
                download_url: String::new(),
                digest: None,
            }],
        }
    }

    fn info(tag: &str) -> Availability {
        Availability::Release(Box::new(ReleaseInfo::evaluate(
            &release_with_asset(tag),
            &windows_target(),
            &["vncli".to_owned()],
            &["zip".to_owned()],
        )))
    }

    #[test]
    fn an_absent_app_with_a_build_is_available() {
        let status = compute(&InstallState::Absent, &info("v0.1.0"), AppKind::Release);
        assert_eq!(
            status,
            AppStatus::Available {
                version: "0.1.0".to_owned(),
                tag: "v0.1.0".to_owned()
            }
        );
        assert!(status.is_installable());
    }

    #[test]
    fn a_newer_release_is_an_update() {
        let install = InstallState::Managed {
            version: "0.1.0".to_owned(),
            path: None,
        };
        assert_eq!(
            compute(&install, &info("v0.2.0"), AppKind::Release),
            AppStatus::UpdateAvailable {
                installed: "0.1.0".to_owned(),
                latest: "0.2.0".to_owned(),
                tag: "v0.2.0".to_owned(),
            }
        );
    }

    #[test]
    fn the_same_version_is_up_to_date_and_an_older_one_is_not_an_update() {
        let install = InstallState::Managed {
            version: "0.2.0".to_owned(),
            path: None,
        };
        assert_eq!(
            compute(&install, &info("v0.2.0"), AppKind::Release),
            AppStatus::UpToDate
        );
        assert_eq!(
            compute(&install, &info("v0.1.0"), AppKind::Release),
            AppStatus::UpToDate
        );
    }

    #[test]
    fn an_incomparable_installed_version_is_never_reported_as_an_update() {
        let install = InstallState::Managed {
            version: "built-from-source".to_owned(),
            path: None,
        };
        match compute(&install, &info("v0.2.0"), AppKind::Release) {
            AppStatus::InstalledUncomparable {
                installed, latest, ..
            } => {
                assert_eq!(installed, "built-from-source");
                assert_eq!(latest, "0.2.0");
            }
            other => panic!("expected InstalledUncomparable, got {other:?}"),
        }
    }

    #[test]
    fn an_app_found_on_the_machine_is_not_offered_for_install() {
        let found = InstallState::Found { path: None };
        assert_eq!(
            compute(&found, &info("v0.1.0"), AppKind::Release),
            AppStatus::Found { version: None }
        );
    }

    #[test]
    fn a_release_with_no_build_for_this_system_says_so() {
        // vncli publishes nothing for macOS.
        let macos = Target {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        let availability = Availability::Release(Box::new(ReleaseInfo::evaluate(
            &release_with_asset("v0.1.0"),
            &macos,
            &["vncli".to_owned()],
            &["zip".to_owned()],
        )));
        assert_eq!(
            compute(&InstallState::Absent, &availability, AppKind::Release),
            AppStatus::NoBuildForThisSystem {
                tag: "v0.1.0".to_owned()
            }
        );
    }

    #[test]
    fn no_build_is_reported_even_when_the_app_is_installed() {
        let install = InstallState::Managed {
            version: "0.1.0".to_owned(),
            path: None,
        };
        let macos = Target {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        let availability = Availability::Release(Box::new(ReleaseInfo::evaluate(
            &release_with_asset("v0.1.0"),
            &macos,
            &["vncli".to_owned()],
            &["zip".to_owned()],
        )));
        assert!(matches!(
            compute(&install, &availability, AppKind::Release),
            AppStatus::NoBuildForThisSystem { .. }
        ));
    }

    #[test]
    fn a_link_entry_has_nothing_to_install() {
        assert_eq!(
            compute(&InstallState::Absent, &Availability::Unknown, AppKind::Link),
            AppStatus::Link
        );
    }

    #[test]
    fn an_unreadable_feed_is_reported_with_its_reason() {
        let availability = Availability::Failed("rate limit exceeded".to_owned());
        match compute(&InstallState::Absent, &availability, AppKind::Release) {
            AppStatus::CheckFailed { reason } => assert_eq!(reason, "rate limit exceeded"),
            other => panic!("expected CheckFailed, got {other:?}"),
        }
    }

    #[test]
    fn a_repository_without_releases_says_so() {
        assert_eq!(
            compute(&InstallState::Absent, &Availability::NoReleases, AppKind::Release),
            AppStatus::NoReleases
        );
    }

    #[test]
    fn only_updates_and_failures_ask_for_attention() {
        // An app that was never installed is not an update.
        assert!(
            !AppStatus::Available {
                version: "1.0.0".to_owned(),
                tag: "v1.0.0".to_owned(),
            }
            .needs_attention()
        );
        assert!(!AppStatus::UpToDate.needs_attention());
        assert!(!AppStatus::NoReleases.needs_attention());
        assert!(!AppStatus::Link.needs_attention());

        assert!(
            AppStatus::UpdateAvailable {
                installed: "1.0.0".to_owned(),
                latest: "1.1.0".to_owned(),
                tag: "v1.1.0".to_owned(),
            }
            .needs_attention()
        );
        assert!(
            AppStatus::CheckFailed {
                reason: "offline".to_owned(),
            }
            .needs_attention()
        );
    }
}
