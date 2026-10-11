//! Reading the GitHub releases API into the launcher's own model.
//!
//! The payload is treated as hostile. Every field is optional, every cap is
//! enforced, and a response that is an error object (`{"message": "Not Found"}`)
//! becomes a readable reason rather than a confusing parse failure.

use serde::Deserialize;

use crate::asset::Asset;
use crate::error::{Error, Result};
use crate::version::Tag;

/// How many releases are kept from one response. The launcher only ever shows the
/// newest few; the rest is weight.
pub const MAX_RELEASES: usize = 30;

/// How much of a release's notes are kept, in characters.
pub const MAX_NOTES_CHARS: usize = 4_000;

/// One release, as the launcher understands it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The tag, e.g. `v0.1.0`.
    pub tag: Tag,
    /// The release's title, when it has one distinct from the tag.
    pub name: Option<String>,
    /// The release notes, truncated to [`MAX_NOTES_CHARS`].
    pub notes: Option<String>,
    /// Whether this is an unpublished draft.
    pub draft: bool,
    /// Whether GitHub marks this a pre-release.
    pub prerelease: bool,
    /// When it was published, as GitHub's ISO-8601 string.
    pub published_at: Option<String>,
    /// The human-readable release page.
    pub html_url: String,
    /// The files attached to it.
    pub assets: Vec<Asset>,
}

impl Release {
    /// The version to show a person.
    #[must_use]
    pub fn version(&self) -> String {
        self.tag.label()
    }

    /// Whether this release could be offered for installation at all.
    #[must_use]
    pub fn is_offerable(&self, allow_prerelease: bool) -> bool {
        !self.draft && (allow_prerelease || !self.prerelease)
    }
}

/// Parse the JSON body of `GET /repos/{owner}/{repo}/releases`.
///
/// # Errors
/// Returns [`Error::ReleasePayload`] when the body is not JSON, or is JSON but not
/// a list of releases — including the API's own `{"message": …}` error shape.
pub fn parse(json: &str) -> Result<Vec<Release>> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| Error::ReleasePayload(format!("not JSON ({e})")))?;

    if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
        return Err(Error::ReleasePayload(message.to_owned()));
    }

    let raw: Vec<RawRelease> =
        serde_json::from_value(value).map_err(|e| Error::ReleasePayload(e.to_string()))?;

    Ok(raw.into_iter().take(MAX_RELEASES).map(convert).collect())
}

/// The newest release that should be offered, or `None` when there is none.
///
/// GitHub returns releases newest-first by creation date, but a repository can
/// publish a patch after a minor. The highest semantic version therefore wins when
/// versions are parseable, and GitHub's own order is the fallback when they are not.
#[must_use]
pub fn select_latest(releases: &[Release], allow_prerelease: bool) -> Option<&Release> {
    let usable: Vec<&Release> = releases
        .iter()
        .filter(|release| release.is_offerable(allow_prerelease))
        .collect();

    let mut best: Option<&Release> = None;
    for release in &usable {
        let Some(version) = release.tag.version() else {
            continue;
        };
        let is_better = best
            .and_then(|current| current.tag.version())
            .is_none_or(|current| version > current);
        if is_better {
            best = Some(release);
        }
    }

    best.or_else(|| usable.first().copied())
}

/// The notes, trimmed for display.
#[must_use]
pub fn notes_excerpt(release: &Release) -> Option<&str> {
    release
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn convert(raw: RawRelease) -> Release {
    Release {
        tag: Tag::parse(&raw.tag_name),
        name: raw.name.filter(|name| !name.trim().is_empty()),
        notes: raw
            .body
            .map(|body| truncate(body, MAX_NOTES_CHARS))
            .filter(|body| !body.trim().is_empty()),
        draft: raw.draft,
        prerelease: raw.prerelease,
        published_at: raw.published_at,
        html_url: raw.html_url.unwrap_or_default(),
        assets: raw
            .assets
            .into_iter()
            .filter(|asset| !asset.name.is_empty())
            .map(|asset| Asset {
                name: asset.name,
                size: asset.size,
                download_url: asset.browser_download_url,
                digest: asset.digest,
            })
            .collect(),
    }
}

/// Cut a string to `max` characters without splitting one.
fn truncate(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }
    let mut out: String = text.chars().take(max).collect();
    out.push('…');
    out
}

#[derive(Debug, Deserialize)]
struct RawRelease {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Debug, Deserialize)]
struct RawAsset {
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    const VNCLI: &str = include_str!("../tests/data/vncli-releases.json");

    #[test]
    fn reads_the_real_vncli_payload() {
        let releases = parse(VNCLI).unwrap();
        assert_eq!(releases.len(), 1);
        let release = &releases[0];
        assert_eq!(release.tag.raw(), "v0.1.0");
        assert_eq!(release.version(), "0.1.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
        assert_eq!(release.assets.len(), 2);
        assert_eq!(release.assets[0].size, 5_133_904);
        assert_eq!(
            release.assets[0].digest.as_deref(),
            Some("sha256:dc0b25928fb5ca14da80072f3d7da610394270ac8ea405a309bbc6fb525c6910")
        );
        assert!(notes_excerpt(release).is_some());
    }

    #[test]
    fn an_api_error_object_becomes_a_readable_reason() {
        let error = parse(r#"{"message":"Not Found"}"#).unwrap_err();
        assert_eq!(error, Error::ReleasePayload("Not Found".to_owned()));
        assert!(error.to_string().contains("Not Found"));
    }

    #[test]
    fn junk_is_refused_rather_than_guessed_at() {
        assert!(parse("<html>nope</html>").is_err());
        assert!(parse("{}").is_err());
    }

    #[test]
    fn a_missing_release_is_not_a_release() {
        assert!(parse("[]").unwrap().is_empty());
        assert!(select_latest(&[], false).is_none());
    }

    #[test]
    fn drafts_are_never_offered_but_prereleases_can_be() {
        let json = r#"[
            {"tag_name":"v2.0.0","draft":true},
            {"tag_name":"v1.1.0-rc.1","prerelease":true},
            {"tag_name":"v1.0.0"}
        ]"#;
        let releases = parse(json).unwrap();

        // A draft is invisible to the public, so it is never the answer.
        assert_eq!(select_latest(&releases, false).unwrap().tag.raw(), "v1.0.0");
        // With pre-releases allowed, the draft still loses; the release candidate wins.
        assert_eq!(select_latest(&releases, true).unwrap().tag.raw(), "v1.1.0-rc.1");
    }

    #[test]
    fn the_highest_version_wins_over_githubs_order() {
        let json = r#"[
            {"tag_name":"v1.0.1"},
            {"tag_name":"v1.2.0"},
            {"tag_name":"v1.1.0"}
        ]"#;
        let releases = parse(json).unwrap();
        assert_eq!(select_latest(&releases, false).unwrap().tag.raw(), "v1.2.0");
    }

    #[test]
    fn unparseable_tags_fall_back_to_githubs_order() {
        let json = r#"[{"tag_name":"nightly-2026-03"},{"tag_name":"nightly-2026-02"}]"#;
        let releases = parse(json).unwrap();
        assert_eq!(
            select_latest(&releases, false).unwrap().tag.raw(),
            "nightly-2026-03"
        );
    }

    #[test]
    fn a_missing_field_never_fails_a_release() {
        let releases = parse(r#"[{"tag_name":"v1.0.0"}]"#).unwrap();
        let release = &releases[0];
        assert!(release.name.is_none());
        assert!(release.assets.is_empty());
        assert_eq!(release.html_url, "");
    }

    #[test]
    fn long_notes_are_cut_on_a_character_boundary() {
        let long = "é".repeat(MAX_NOTES_CHARS + 10);
        let json = format!(r#"[{{"tag_name":"v1.0.0","body":"{long}"}}]"#);
        let releases = parse(&json).unwrap();
        let notes = releases[0].notes.as_deref().unwrap();
        assert_eq!(notes.chars().count(), MAX_NOTES_CHARS + 1);
        assert!(notes.ends_with('…'));
    }

    #[test]
    fn the_number_of_releases_is_capped() {
        let json = format!(
            "[{}]",
            (0..MAX_RELEASES + 20)
                .map(|i| format!(r#"{{"tag_name":"v0.{i}.0"}}"#))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(parse(&json).unwrap().len(), MAX_RELEASES);
    }
}
