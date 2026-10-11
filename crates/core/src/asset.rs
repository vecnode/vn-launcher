//! Choosing the one release asset that belongs on this machine.
//!
//! This is the rule that matters most in the whole launcher: downloading the
//! wrong build is worse than downloading nothing. The rules here are therefore
//! conservative — an asset that names another operating system or another CPU
//! architecture is rejected outright, and if two assets fit equally well the
//! launcher refuses to guess and asks the person instead.
//!
//! Names are matched on *tokens*, never on substrings. That distinction is not
//! academic: `darwin` contains `win`, so a naive `contains("win")` would offer a
//! macOS build to a Windows machine.

use crate::platform::{Arch, Os, Target};

/// Archive and package extensions the launcher will consider.
///
/// Ordered longest-first so `tar.gz` is recognised before any bare `gz` rule
/// could apply.
const EXTENSIONS: &[&str] = &[
    "tar.gz", "tar.xz", "tar.bz2", "tar.zst", "tgz", "zip", "7z", "dmg", "pkg", "msi", "exe", "deb", "rpm",
    "appimage", "flatpak",
];

/// Score for an asset whose name begins with a declared stem.
const SCORE_STEM: u32 = 16;
/// Score for an asset that names this operating system.
const SCORE_OS: u32 = 8;
/// Score for an asset that names this architecture.
const SCORE_ARCH: u32 = 4;
/// Score for an asset that names no architecture, and so is universal.
const SCORE_ARCH_UNIVERSAL: u32 = 2;

/// One downloadable file attached to a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// The file name as published.
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// The URL the file is fetched from.
    pub download_url: String,
    /// GitHub's own `sha256:` digest of the file, when it reports one.
    pub digest: Option<String>,
}

impl Asset {
    /// The size, rendered for a person.
    #[must_use]
    pub fn human_size(&self) -> String {
        human_size(self.size)
    }
}

/// What the matching rules concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetChoice {
    /// Exactly one asset fits this machine.
    Chosen(Asset),
    /// The release publishes nothing for this system.
    None,
    /// Several assets fit equally well; the names are listed so a person can choose.
    Ambiguous(Vec<String>),
}

impl AssetChoice {
    /// The chosen asset, if there is exactly one.
    #[must_use]
    pub fn chosen(&self) -> Option<&Asset> {
        match self {
            Self::Chosen(asset) => Some(asset),
            Self::None | Self::Ambiguous(_) => None,
        }
    }
}

/// Pick the asset for `target`.
///
/// `stems` are the file-name prefixes an app's releases are known to use, which is
/// how a repository whose assets are not named after the repository is still
/// matched. `extensions` is the preference order; an empty slice means "whatever
/// this operating system normally ships".
#[must_use]
pub fn choose(assets: &[Asset], target: &Target, stems: &[String], extensions: &[String]) -> AssetChoice {
    let fallback: Vec<String> = target
        .os
        .default_extensions()
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let preference: &[String] = if extensions.is_empty() {
        &fallback
    } else {
        extensions
    };

    let mut scored: Vec<(u32, &Asset)> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    for asset in assets {
        // A release may attach the same file twice; that is not an ambiguity.
        if seen.contains(&asset.name.as_str()) {
            continue;
        }
        seen.push(&asset.name);

        let Some(extension) = extension(&asset.name) else {
            continue;
        };
        let Some(rank) = preference.iter().position(|p| p.eq_ignore_ascii_case(extension)) else {
            continue;
        };

        let lower = asset.name.to_ascii_lowercase();
        let words = tokens(&lower);

        let named_os = detect_os(&words);
        match named_os {
            Some(os) if os != target.os => continue,
            _ => {}
        }
        let named_arch = detect_arch(&words);
        match named_arch {
            Some(arch) if arch != target.arch => continue,
            _ => {}
        }

        let mut score = preference.len().saturating_sub(rank) as u32;
        if named_os.is_some() {
            score += SCORE_OS;
        }
        score += if named_arch.is_some() {
            SCORE_ARCH
        } else {
            SCORE_ARCH_UNIVERSAL
        };
        if stems.iter().any(|stem| stem_matches(&lower, stem)) {
            score += SCORE_STEM;
        }

        scored.push((score, asset));
    }

    let Some(best) = scored.iter().map(|(score, _)| *score).max() else {
        return AssetChoice::None;
    };

    let winners: Vec<&Asset> = scored
        .iter()
        .filter(|(score, _)| *score == best)
        .map(|(_, asset)| *asset)
        .collect();

    if winners.len() == 1 {
        AssetChoice::Chosen(winners[0].clone())
    } else {
        AssetChoice::Ambiguous(winners.iter().map(|asset| asset.name.clone()).collect())
    }
}

/// Whether an extension is one the launcher knows how to unpack.
///
/// A catalog may only ask for extensions on this list; anything else is dropped
/// rather than silently producing a download nothing can open.
#[must_use]
pub fn is_known_extension(value: &str) -> bool {
    let value = value.trim().trim_start_matches('.');
    EXTENSIONS.iter().any(|ext| ext.eq_ignore_ascii_case(value))
}

/// The recognised extension of a file name, if any.
fn extension(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    EXTENSIONS
        .iter()
        .copied()
        .find(|ext| lower.strip_suffix(ext).is_some_and(|rest| rest.ends_with('.')))
}

/// Split a name into lower-case words on every non-alphanumeric character.
fn tokens(lower_name: &str) -> Vec<String> {
    lower_name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Whether an asset name begins with a declared stem.
fn stem_matches(lower_name: &str, stem: &str) -> bool {
    let stem = stem.to_ascii_lowercase();
    lower_name == stem
        || lower_name
            .strip_prefix(&stem)
            .is_some_and(|rest| matches!(rest.chars().next(), Some('-' | '_' | '.' | ' ')))
}

/// Which operating system an asset name claims to be for.
fn detect_os(words: &[String]) -> Option<Os> {
    let has = |word: &str| words.iter().any(|w| w == word);
    if has("windows") || has("win32") || has("win64") || has("win") || has("msvc") || has("mingw") {
        Some(Os::Windows)
    } else if has("darwin") || has("macos") || has("osx") || has("mac") || has("apple") {
        Some(Os::MacOs)
    } else if has("linux") {
        Some(Os::Linux)
    } else {
        None
    }
}

/// Which architecture an asset name claims to be for.
fn detect_arch(words: &[String]) -> Option<Arch> {
    let has = |word: &str| words.iter().any(|w| w == word);
    let adjacent = |a: &str, b: &str| words.windows(2).any(|pair| pair[0] == a && pair[1] == b);

    if has("aarch64") || has("arm64") {
        Some(Arch::Aarch64)
    } else if has("x86_64") || has("amd64") || has("x64") || adjacent("x86", "64") {
        // `x86_64` and `x86-64` both split into the words `x86` and `64`.
        Some(Arch::X86_64)
    } else if has("i686") || has("i386") || has("x86") || has("win32") {
        Some(Arch::X86)
    } else {
        None
    }
}

/// Render a byte count for a person.
#[must_use]
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.to_owned(),
            size: 1_000_000,
            download_url: format!("https://example.invalid/{name}"),
            digest: None,
        }
    }

    fn target(os: Os, arch: Arch) -> Target {
        Target { os, arch }
    }

    fn stems(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    fn prefs(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    /// The real asset list of `vecnode/vncli` v0.1.0.
    fn vncli_assets() -> Vec<Asset> {
        vec![
            asset("vncli-0.1.0-x86_64-pc-windows-msvc.zip"),
            asset("vncli-0.1.0-x86_64-unknown-linux-gnu.tar.gz"),
        ]
    }

    /// The real asset list of `vecnode/vncode` v0.1.0.
    fn vncode_assets() -> Vec<Asset> {
        vec![
            asset("SHA256SUMS.txt"),
            asset("vncode-0.1.0-linux.zip"),
            asset("vncode-0.1.0-mac.zip"),
            asset("vncode-0.1.0-win.zip"),
        ]
    }

    #[test]
    fn finds_the_windows_build_of_vncli() {
        let choice = choose(
            &vncli_assets(),
            &target(Os::Windows, Arch::X86_64),
            &stems(&["vncli"]),
            &prefs(&["zip", "tar.gz"]),
        );
        assert_eq!(
            choice.chosen().map(|a| a.name.as_str()),
            Some("vncli-0.1.0-x86_64-pc-windows-msvc.zip")
        );
    }

    #[test]
    fn finds_the_linux_build_of_vncli() {
        let choice = choose(
            &vncli_assets(),
            &target(Os::Linux, Arch::X86_64),
            &stems(&["vncli"]),
            &prefs(&["zip", "tar.gz"]),
        );
        assert_eq!(
            choice.chosen().map(|a| a.name.as_str()),
            Some("vncli-0.1.0-x86_64-unknown-linux-gnu.tar.gz")
        );
    }

    #[test]
    fn says_so_plainly_when_there_is_no_build_for_this_system() {
        // vncli publishes no macOS asset; the launcher must not offer one.
        let choice = choose(
            &vncli_assets(),
            &target(Os::MacOs, Arch::Aarch64),
            &stems(&["vncli"]),
            &prefs(&["zip", "tar.gz"]),
        );
        assert_eq!(choice, AssetChoice::None);
    }

    #[test]
    fn never_offers_a_checksum_file_or_another_platforms_build() {
        for (os, arch, expected) in [
            (Os::Windows, Arch::X86_64, "vncode-0.1.0-win.zip"),
            (Os::MacOs, Arch::Aarch64, "vncode-0.1.0-mac.zip"),
            (Os::Linux, Arch::X86_64, "vncode-0.1.0-linux.zip"),
        ] {
            let choice = choose(
                &vncode_assets(),
                &target(os, arch),
                &stems(&["vncode"]),
                &prefs(&["zip", "tar.gz"]),
            );
            assert_eq!(choice.chosen().map(|a| a.name.as_str()), Some(expected), "{os:?}");
        }
    }

    #[test]
    fn darwin_is_not_windows() {
        // The regression that motivates token matching: `darwin` contains `win`.
        let assets = vec![asset("app-1.0.0-darwin-arm64.tar.gz")];
        let choice = choose(
            &assets,
            &target(Os::Windows, Arch::X86_64),
            &stems(&["app"]),
            &prefs(&["zip", "tar.gz"]),
        );
        assert_eq!(choice, AssetChoice::None);
    }

    #[test]
    fn refuses_to_guess_between_two_equally_good_assets() {
        let assets = vec![
            asset("app-1.0.0-windows-x86_64.zip"),
            asset("app-1.0.0-win-x86_64.zip"),
        ];
        let choice = choose(
            &assets,
            &target(Os::Windows, Arch::X86_64),
            &stems(&["app"]),
            &prefs(&["zip"]),
        );
        match choice {
            AssetChoice::Ambiguous(names) => assert_eq!(names.len(), 2),
            other => panic!("expected an ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn a_generic_build_loses_to_a_platform_specific_one() {
        let assets = vec![asset("app-1.0.0.zip"), asset("app-1.0.0-linux-x86_64.zip")];
        let choice = choose(
            &assets,
            &target(Os::Linux, Arch::X86_64),
            &stems(&["app"]),
            &prefs(&["zip"]),
        );
        assert_eq!(
            choice.chosen().map(|a| a.name.as_str()),
            Some("app-1.0.0-linux-x86_64.zip")
        );
    }

    #[test]
    fn a_stem_that_is_not_the_repository_name_still_matches() {
        // `p5js-window` publishes assets named `neo-processing-*`.
        let assets = vec![
            asset("neo-processing-windows.zip"),
            asset("neo-processing-linux.tar.gz"),
        ];
        let choice = choose(
            &assets,
            &target(Os::Linux, Arch::X86_64),
            &stems(&["neo-processing"]),
            &prefs(&["zip", "tar.gz"]),
        );
        assert_eq!(
            choice.chosen().map(|a| a.name.as_str()),
            Some("neo-processing-linux.tar.gz")
        );
    }

    #[test]
    fn an_extension_that_was_not_asked_for_is_ignored() {
        let assets = vec![asset("app-1.0.0-linux-x86_64.deb")];
        let choice = choose(
            &assets,
            &target(Os::Linux, Arch::X86_64),
            &stems(&["app"]),
            &prefs(&["tar.gz"]),
        );
        assert_eq!(choice, AssetChoice::None);
    }

    #[test]
    fn duplicate_attachments_are_not_an_ambiguity() {
        let assets = vec![
            asset("app-1.0.0-windows-x86_64.zip"),
            asset("app-1.0.0-windows-x86_64.zip"),
        ];
        let choice = choose(
            &assets,
            &target(Os::Windows, Arch::X86_64),
            &stems(&["app"]),
            &prefs(&["zip"]),
        );
        assert!(choice.chosen().is_some());
    }

    #[test]
    fn extension_detection_handles_two_part_suffixes() {
        assert_eq!(extension("a.tar.gz"), Some("tar.gz"));
        assert_eq!(extension("a.zip"), Some("zip"));
        assert_eq!(extension("SHA256SUMS.txt"), None);
        assert_eq!(extension("foozip"), None);
        assert_eq!(extension("App.AppImage"), Some("appimage"));
    }

    #[test]
    fn arch_detection_handles_the_spellings_in_use() {
        assert_eq!(detect_arch(&tokens("a-x86_64")), Some(Arch::X86_64));
        assert_eq!(detect_arch(&tokens("a-amd64")), Some(Arch::X86_64));
        assert_eq!(detect_arch(&tokens("a-x64")), Some(Arch::X86_64));
        assert_eq!(detect_arch(&tokens("a-arm64")), Some(Arch::Aarch64));
        assert_eq!(detect_arch(&tokens("a-aarch64")), Some(Arch::Aarch64));
        assert_eq!(detect_arch(&tokens("a-i686")), Some(Arch::X86));
        assert_eq!(detect_arch(&tokens("a-universal")), None);
    }

    #[test]
    fn sizes_read_well() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(5_133_904), "5.1 MB");
        assert_eq!(human_size(1_500_000_000), "1.5 GB");
    }
}
