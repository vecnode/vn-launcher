//! A GitHub repository reference, and the URLs derived from it.
//!
//! The launcher never authenticates, so a repository is nothing more than an
//! `owner/name` pair plus the public URLs built from it. Parsing is deliberately
//! forgiving about the shapes people paste (a browser URL, a clone URL, an SSH
//! URL, a deep link into a release) and deliberately strict about what may end
//! up in a name, because these strings become filesystem paths.

use core::fmt;

use crate::error::{Error, Result};

/// Longest legal GitHub owner or repository name.
const MAX_NAME: usize = 100;

/// A public GitHub repository, `owner/name`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Repo {
    owner: String,
    name: String,
}

impl Repo {
    /// Parse anything a person might paste for a repository.
    ///
    /// Accepted: `owner/name`, `github.com/owner/name`, `https://github.com/owner/name`,
    /// a clone URL ending in `.git`, an SSH URL `git@github.com:owner/name.git`, and any of
    /// those with a trailing path such as `/releases` or `/tree/main` or a `?query#fragment`.
    ///
    /// # Errors
    /// Returns [`Error::BadRepo`] when the value does not name a repository at all, and
    /// [`Error::BadRepoPart`] when a name is empty, too long, or contains a character
    /// GitHub does not allow (which also rejects anything resembling a path traversal).
    pub fn parse(input: &str) -> Result<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(Error::BadRepo(input.to_owned()));
        }

        // Drop any fragment or query, then the scheme and host.
        let without_fragment = raw.split(['#', '?']).next().unwrap_or(raw);
        let without_scheme = without_fragment
            .strip_prefix("https://")
            .or_else(|| without_fragment.strip_prefix("http://"))
            .or_else(|| without_fragment.strip_prefix("ssh://"))
            .unwrap_or(without_fragment);
        // `git@github.com:owner/name` -> `github.com/owner/name`
        let without_user = without_scheme
            .split_once('@')
            .map_or(without_scheme, |(_, rest)| rest);
        let normalised = without_user.replace(':', "/");
        let path = normalised
            .strip_prefix("github.com/")
            .or_else(|| normalised.strip_prefix("www.github.com/"))
            .unwrap_or(&normalised);

        let mut segments = path.split('/').filter(|s| !s.is_empty());
        let owner = segments.next().ok_or_else(|| Error::BadRepo(input.to_owned()))?;
        let name = segments.next().ok_or_else(|| Error::BadRepo(input.to_owned()))?;
        let name = name.strip_suffix(".git").unwrap_or(name);

        let owner = validate("owner", owner)?;
        let name = validate("repository", name)?;
        Ok(Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    /// The owner (user or organisation).
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `owner/name`, the form used in the catalog and in log lines.
    #[must_use]
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// The repository's page.
    #[must_use]
    pub fn html_url(&self) -> String {
        format!("https://github.com/{}", self.slug())
    }

    /// The repository's Releases page, which is what a person should be sent to when
    /// the launcher cannot install an app itself.
    #[must_use]
    pub fn releases_url(&self) -> String {
        format!("{}/releases", self.html_url())
    }

    /// The REST endpoint listing releases, newest first.
    #[must_use]
    pub fn api_releases_url(&self) -> String {
        format!("https://api.github.com/repos/{}/releases", self.slug())
    }

    /// The REST endpoint describing the repository itself.
    #[must_use]
    pub fn api_repo_url(&self) -> String {
        format!("https://api.github.com/repos/{}", self.slug())
    }

    /// The endpoint listing tags, used when a repository publishes no releases.
    #[must_use]
    pub fn api_tags_url(&self) -> String {
        format!("https://api.github.com/repos/{}/tags", self.slug())
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.slug())
    }
}

/// Check a single GitHub name.
fn validate<'a>(part: &'static str, value: &'a str) -> Result<&'a str> {
    let legal = !value.is_empty()
        && value.len() <= MAX_NAME
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if legal {
        Ok(value)
    } else {
        Err(Error::BadRepoPart {
            part,
            value: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn parses_every_shape_a_person_pastes() {
        for input in [
            "vecnode/vncli",
            "vecnode/vncli/",
            "github.com/vecnode/vncli",
            "https://github.com/vecnode/vncli",
            "https://github.com/vecnode/vncli.git",
            "https://github.com/vecnode/vncli/releases",
            "https://github.com/vecnode/vncli/tree/main",
            "https://github.com/vecnode/vncli?tab=readme",
            "https://github.com/vecnode/vncli#readme",
            "git@github.com:vecnode/vncli.git",
            "  vecnode/vncli  ",
        ] {
            let repo = Repo::parse(input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(repo.slug(), "vecnode/vncli", "input {input}");
        }
    }

    #[test]
    fn rejects_traversal_and_junk() {
        for input in [
            "",
            "vncli",
            "vecnode/",
            "/vncli",
            "vecnode/../etc",
            "vecnode/..",
            "vecnode/.",
            "owner/na me",
            "owner/na/me/too/deep",
        ] {
            // `owner/na/me/too/deep` parses `owner/na` and ignores the tail, which is
            // correct for a deep link; everything else must fail.
            if input == "owner/na/me/too/deep" {
                assert!(Repo::parse(input).is_ok());
            } else {
                assert!(Repo::parse(input).is_err(), "should reject {input:?}");
            }
        }
    }

    #[test]
    fn urls_are_the_public_ones() {
        let repo = Repo::parse("vecnode/vncli").unwrap();
        assert_eq!(repo.html_url(), "https://github.com/vecnode/vncli");
        assert_eq!(repo.releases_url(), "https://github.com/vecnode/vncli/releases");
        assert_eq!(
            repo.api_releases_url(),
            "https://api.github.com/repos/vecnode/vncli/releases"
        );
    }

    #[test]
    fn very_long_names_are_refused() {
        let long = "a".repeat(MAX_NAME + 1);
        assert!(Repo::parse(&format!("owner/{long}")).is_err());
    }
}
