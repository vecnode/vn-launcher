//! Parsing `SHA256SUMS.txt`, the checksum file the vecnode repositories publish.
//!
//! Only the parsing lives here — it is pure text work with no hashing. Verifying a
//! download against these digests belongs to the install step, which the launcher
//! does not yet perform; see `docs/RELEASES.md` for the convention itself.

/// The digests listed in one `SHA256SUMS.txt`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Checksums {
    /// `(lower-case hex digest, file name)` pairs, in file order.
    entries: Vec<(String, String)>,
}

impl Checksums {
    /// How many files are listed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing was listed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The digest recorded for a file name, if it is listed.
    ///
    /// Lookup is by exact name first, then by the name with any leading `./` or
    /// directory prefix removed, because both spellings appear in the wild.
    #[must_use]
    pub fn digest_of(&self, file_name: &str) -> Option<&str> {
        let wanted = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
        self.entries
            .iter()
            .find(|(_, name)| name == file_name)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|(_, name)| name.rsplit(['/', '\\']).next() == Some(wanted))
            })
            .map(|(digest, _)| digest.as_str())
    }

    /// Every listed file name.
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(_, name)| name.as_str())
    }
}

/// Parse the contents of a `SHA256SUMS.txt`.
///
/// Accepts the GNU coreutils format (two spaces between digest and name), the
/// binary-mode marker `*`, a leading `./`, and blank lines or `#` comments.
/// Anything whose first field is not a 64-character hexadecimal digest is skipped.
#[must_use]
pub fn parse(text: &str) -> Checksums {
    let mut entries = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((digest, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !is_sha256_hex(digest) {
            continue;
        }
        let name = rest.trim_start().trim_start_matches('*').trim();
        if name.is_empty() {
            continue;
        }
        let name = name.strip_prefix("./").unwrap_or(name);
        entries.push((digest.to_ascii_lowercase(), name.to_owned()));
    }

    Checksums { entries }
}

/// Whether a string is exactly 64 hexadecimal characters.
#[must_use]
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Whether a release asset name is a checksum file rather than a build.
///
/// Used so a `SHA256SUMS.txt` is never mistaken for something installable.
#[must_use]
pub fn is_checksum_asset(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("sha256sums")
        || lower.contains("checksums")
        || lower.ends_with(".sha256")
        || lower.ends_with(".sha256sum")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    const SAMPLE: &str = "\
# vncode 0.1.0
a4a9447b09e37a417c0fb90e7c826c46801c6b32eaf49497b78bbf1842a6b990  vncode-0.1.0-linux.zip
9352E076A11F6EFA84AC1570D44616F809A509FB39EABA5E141A6C12F3391FBD *vncode-0.1.0-mac.zip
c5336cb217929deb578530827d172f09e3896aefa99b1df28a56768992209a49  ./vncode-0.1.0-win.zip

not-a-digest  ignored.bin
";

    #[test]
    fn reads_the_gnu_and_binary_marker_forms() {
        let sums = parse(SAMPLE);
        assert_eq!(sums.len(), 3);
        assert_eq!(
            sums.digest_of("vncode-0.1.0-linux.zip"),
            Some("a4a9447b09e37a417c0fb90e7c826c46801c6b32eaf49497b78bbf1842a6b990")
        );
        // `*` marker and upper case are both handled.
        assert_eq!(
            sums.digest_of("vncode-0.1.0-mac.zip"),
            Some("9352e076a11f6efa84ac1570d44616f809a509fb39eaba5e141a6c12f3391fbd")
        );
        // `./` prefix is stripped on parse.
        assert_eq!(
            sums.digest_of("vncode-0.1.0-win.zip"),
            Some("c5336cb217929deb578530827d172f09e3896aefa99b1df28a56768992209a49")
        );
    }

    #[test]
    fn junk_lines_are_skipped_rather_than_rejected() {
        assert!(parse("").is_empty());
        assert!(parse("garbage\n\n# comment\n").is_empty());
        assert!(parse("abc  short-digest.zip").is_empty());
    }

    #[test]
    fn a_file_that_is_not_listed_has_no_digest() {
        assert!(parse(SAMPLE).digest_of("other.zip").is_none());
    }

    #[test]
    fn a_path_is_matched_by_its_file_name() {
        let sums = parse("a4a9447b09e37a417c0fb90e7c826c46801c6b32eaf49497b78bbf1842a6b990  dist/app.zip");
        assert!(sums.digest_of("app.zip").is_some());
        assert!(sums.digest_of("dist/app.zip").is_some());
    }

    #[test]
    fn checksum_files_are_recognised() {
        assert!(is_checksum_asset("SHA256SUMS.txt"));
        assert!(is_checksum_asset("sha256sums"));
        assert!(is_checksum_asset("app.zip.sha256"));
        assert!(!is_checksum_asset("vncli-0.1.0-x86_64-pc-windows-msvc.zip"));
    }

    #[test]
    fn hexadecimal_validation_is_exact() {
        assert!(is_sha256_hex(&"a".repeat(64)));
        assert!(!is_sha256_hex(&"a".repeat(63)));
        assert!(!is_sha256_hex(&"z".repeat(64)));
    }
}
