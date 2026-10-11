//! Finding an app that is already on this machine.
//!
//! "Keep track of the apps on the computer" starts here. The launcher deliberately
//! distinguishes an app *it* installed from one it merely *found*: a binary that
//! turned up on `PATH` can be launched, but it cannot be updated, because the
//! launcher did not put it there and does not know how it was built.

use std::path::{Path, PathBuf};

use vnl_core::InstallState;
use vnl_core::catalog::DetectSpec;

/// How an app was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// It was not in any of the places the catalog names.
    Nowhere,
    /// A path declared in the catalog exists.
    AtPath(PathBuf),
    /// A command was resolved on `PATH`.
    OnPath(PathBuf),
}

/// The outcome of looking for one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    /// What was found.
    pub found: Found,
    /// Everything that was looked at, in order, for the detail view.
    pub checked: Vec<String>,
}

impl Detection {
    /// Where it was found, if it was.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match &self.found {
            Found::AtPath(path) | Found::OnPath(path) => Some(path),
            Found::Nowhere => None,
        }
    }

    /// Whether it was found at all.
    #[must_use]
    pub fn is_found(&self) -> bool {
        !matches!(self.found, Found::Nowhere)
    }

    /// How to describe this presence to the rest of the launcher.
    ///
    /// Anything found but not installed by the launcher is [`InstallState::Found`],
    /// never [`InstallState::Managed`]: the launcher must not claim ownership of
    /// something it did not install, because that is what would make it believe it
    /// could safely update or remove it.
    #[must_use]
    pub fn to_install_state(&self) -> InstallState {
        match self.path() {
            Some(path) => InstallState::Found {
                path: Some(path.to_string_lossy().into_owned()),
            },
            None => InstallState::Absent,
        }
    }
}

/// Look for an app, using the catalog's [`DetectSpec`].
///
/// `{install}` in a declared path is replaced with the directory the launcher would
/// install the app into.
#[must_use]
pub fn detect(spec: &DetectSpec, app_dir: &Path) -> Detection {
    let mut checked = Vec::new();
    let install = app_dir.to_string_lossy();

    for pattern in &spec.paths {
        let resolved = pattern.replace("{install}", &install);
        checked.push(resolved.clone());
        let candidate = PathBuf::from(&resolved);
        if candidate.is_file() {
            return Detection {
                found: Found::AtPath(candidate),
                checked,
            };
        }
    }

    if let Some(command) = spec.command.as_deref().filter(|c| !c.trim().is_empty()) {
        checked.push(format!("`{command}` on PATH"));
        if let Some(path) = find_on_path(command) {
            return Detection {
                found: Found::OnPath(path),
                checked,
            };
        }
    }

    Detection {
        found: Found::Nowhere,
        checked,
    }
}

/// Resolve a command against `PATH`, honouring `PATHEXT` on Windows.
///
/// This walks `PATH` itself rather than running `which` or `where`, so it cannot
/// hang on a slow subprocess and needs no shell.
///
/// Only **absolute** entries are searched. A relative entry — and an empty one, which
/// every platform reads as "the current directory" — would make the launcher report an
/// app as present because a file of that name happened to sit in whatever directory
/// the launcher was started from, and then offer to launch it. That is the classic
/// untrusted-search-path mistake, and it costs one comparison to avoid.
#[must_use]
pub fn find_on_path(command: &str) -> Option<PathBuf> {
    if command.trim().is_empty() {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        if directory.as_os_str().is_empty() || !directory.is_absolute() {
            continue;
        }
        for candidate in candidates(&directory, command) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// The file names to try inside one `PATH` directory.
///
/// Split by target rather than conditionally mutating one vector. An `#[cfg]` on an
/// `if` statement compiles the mutation away on every other platform, which leaves
/// the binding unused there — and CI denies warnings, so that is not a style nit but
/// a build failure on Linux and macOS that a Windows machine cannot reproduce. Two
/// self-contained bodies make what compiles on a platform the whole of what is there.
#[cfg(windows)]
fn candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
    let mut out = vec![directory.join(command)];

    // `PATHEXT` is how Windows decides that `vn` may mean `vn.exe`.
    if Path::new(command).extension().is_none() {
        let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
        for extension in extensions.split(';').filter(|e| !e.trim().is_empty()) {
            out.push(directory.join(format!("{command}{}", extension.trim().to_lowercase())));
        }
    }

    out
}

/// Everywhere else a command is exactly its own name.
#[cfg(not(windows))]
fn candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
    vec![directory.join(command)]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_declared_path_that_exists_is_found() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("tool");
        std::fs::write(&binary, b"#!/bin/sh\n").unwrap();

        let spec = DetectSpec {
            paths: vec!["{install}/tool".to_owned()],
            command: None,
        };
        let detection = detect(&spec, temp.path());
        assert!(detection.is_found());
        assert_eq!(detection.path(), Some(binary.as_path()));
        assert!(matches!(detection.found, Found::AtPath(_)));
    }

    #[test]
    fn what_was_looked_at_is_reported_even_on_a_miss() {
        let temp = tempfile::tempdir().unwrap();
        let spec = DetectSpec {
            paths: vec!["{install}/nope".to_owned()],
            command: Some("definitely-not-a-real-command-xyz".to_owned()),
        };
        let detection = detect(&spec, temp.path());
        assert!(!detection.is_found());
        assert_eq!(detection.found, Found::Nowhere);
        assert_eq!(detection.checked.len(), 2);
        assert!(detection.checked[0].contains("nope"));
        assert!(detection.checked[1].contains("on PATH"));
    }

    #[test]
    fn a_found_app_is_never_claimed_as_managed() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("tool");
        std::fs::write(&binary, b"x").unwrap();
        let spec = DetectSpec {
            paths: vec!["{install}/tool".to_owned()],
            command: None,
        };
        let state = detect(&spec, temp.path()).to_install_state();
        assert!(matches!(state, InstallState::Found { .. }));
        assert!(!state.is_managed());
    }

    #[test]
    fn nothing_found_is_absent() {
        let temp = tempfile::tempdir().unwrap();
        let spec = DetectSpec::default();
        assert_eq!(
            detect(&spec, temp.path()).to_install_state(),
            InstallState::Absent
        );
    }

    #[test]
    fn a_command_is_resolved_against_a_path_we_control() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join(if cfg!(windows) {
            "vnl-probe.exe"
        } else {
            "vnl-probe"
        });
        std::fs::write(&binary, b"x").unwrap();

        // `find_on_path` reads the real environment, so this asserts on the
        // candidate-naming rule rather than mutating process-wide state.
        let named = candidates(temp.path(), "vnl-probe");
        assert!(named.contains(&temp.path().join("vnl-probe")));
        // `cfg!` rather than `#[cfg]`: an attribute on a statement deletes it on every
        // other platform, which is the shape that broke the build on Linux and macOS.
        // A condition is compiled everywhere and simply not taken.
        if cfg!(windows) {
            assert!(
                named
                    .iter()
                    .any(|path| path.to_string_lossy().ends_with("vnl-probe.exe")),
                "PATHEXT candidates must be tried on Windows"
            );
        }
    }

    #[test]
    fn an_empty_command_is_not_looked_for() {
        assert!(find_on_path("").is_none());
        assert!(find_on_path("   ").is_none());
    }
}
