//! Release tags, understood as much as they honestly can be.
//!
//! The vecnode repositories do not all tag the same way, and some do not tag at
//! all. A tag is therefore modelled as a raw string that *may* turn out to be a
//! semantic version. When it does, versions compare properly; when it does not,
//! the launcher says "version unknown" instead of inventing an ordering, because
//! claiming an update exists when it does not is worse than admitting ignorance.

use core::cmp::Ordering;

use semver::Version;

/// A release tag: the raw label GitHub reports, plus its meaning if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    raw: String,
    parsed: Option<Version>,
}

impl Tag {
    /// Read a tag, tolerating a leading `v` or `V`.
    ///
    /// `v0.1.0`, `V0.1.0` and `0.1.0` all yield the version `0.1.0`; anything else
    /// (`nightly`, `release-2026`, `v2`) is kept as an opaque label.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let candidate = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);
        let parsed = Version::parse(candidate)
            .ok()
            .or_else(|| Version::parse(trimmed).ok());
        Self {
            raw: trimmed.to_owned(),
            parsed,
        }
    }

    /// The tag exactly as GitHub reports it.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The version, when the tag is one.
    #[must_use]
    pub fn version(&self) -> Option<&Version> {
        self.parsed.as_ref()
    }

    /// Whether the tag is a semantic version at all.
    #[must_use]
    pub fn is_version(&self) -> bool {
        self.parsed.is_some()
    }

    /// What to show a person: `0.1.0` for `v0.1.0`, otherwise the raw tag.
    #[must_use]
    pub fn label(&self) -> String {
        self.parsed
            .as_ref()
            .map_or_else(|| self.raw.clone(), ToString::to_string)
    }

    /// Whether this is a pre-release (a semantic version carrying a pre-release part).
    #[must_use]
    pub fn is_prerelease(&self) -> bool {
        self.parsed.as_ref().is_some_and(|v| !v.pre.is_empty())
    }
}

impl core::fmt::Display for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.label())
    }
}

/// Compare two tags, but only when both are versions.
///
/// Returns `None` when either side is an opaque label, which is the signal to the
/// caller that no update decision can be made.
#[must_use]
pub fn compare(a: &Tag, b: &Tag) -> Option<Ordering> {
    match (a.version(), b.version()) {
        (Some(a), Some(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

/// Whether `candidate` is strictly newer than `installed`.
///
/// `None` means the two cannot be compared, so the caller must not claim an update.
#[must_use]
pub fn is_newer(candidate: &Tag, installed: &Tag) -> Option<bool> {
    compare(candidate, installed).map(|ordering| ordering == Ordering::Greater)
}

/// Strip a display-only prefix from a version string.
#[must_use]
pub fn clean(raw: &str) -> String {
    Tag::parse(raw).label()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn reads_the_prefixes_the_repos_actually_use() {
        for raw in ["v0.1.0", "V0.1.0", "0.1.0"] {
            let tag = Tag::parse(raw);
            assert!(tag.is_version(), "{raw}");
            assert_eq!(tag.label(), "0.1.0", "{raw}");
        }
        assert_eq!(Tag::parse("v0.1.0").raw(), "v0.1.0");
    }

    #[test]
    fn refuses_to_invent_an_ordering_for_labels() {
        let nightly = Tag::parse("nightly");
        assert!(!nightly.is_version());
        assert_eq!(nightly.label(), "nightly");
        assert!(compare(&Tag::parse("v0.2.0"), &nightly).is_none());
        assert!(is_newer(&Tag::parse("v0.2.0"), &nightly).is_none());
    }

    #[test]
    fn compares_real_versions() {
        let older = Tag::parse("v0.1.0");
        let newer = Tag::parse("v0.2.0");
        assert_eq!(is_newer(&newer, &older), Some(true));
        assert_eq!(is_newer(&older, &newer), Some(false));
        assert_eq!(is_newer(&older, &Tag::parse("0.1.0")), Some(false));
    }

    #[test]
    fn a_prerelease_is_older_than_its_release() {
        let beta = Tag::parse("v1.0.0-beta.1");
        let stable = Tag::parse("v1.0.0");
        assert!(beta.is_prerelease());
        assert!(!stable.is_prerelease());
        assert_eq!(is_newer(&stable, &beta), Some(true));
    }

    #[test]
    fn clean_is_the_display_form() {
        assert_eq!(clean("v0.1.0"), "0.1.0");
        assert_eq!(clean("nightly"), "nightly");
    }
}
