//! Taking back what the launcher itself installed.
//!
//! Removal is the one operation in this program that deletes something, so it has one
//! rule and no exceptions: **the launcher deletes only what it put inside its own install
//! root, and never follows a link out of it.** An app it merely found — on `PATH`, in a
//! folder a person pointed it at, built from source — is not the launcher's to remove,
//! which is the same distinction `detect` makes between [`vnl_core::InstallState::Found`]
//! and `Managed`.
//!
//! What that means concretely: `apps/<id>/` under the data directory, and nothing else.
//! The architecture that makes this safe is in `docs/INSTALL.md`; the download, unpack and
//! record half of it is not implemented yet.

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result, io_context};
use crate::paths::Paths;

/// What removing an app's installed files did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    /// The launcher had installed nothing for this app, so nothing was touched.
    NothingThere,
    /// The launcher's own copy was here, and has been deleted.
    Deleted(PathBuf),
}

/// Delete the launcher's own copy of an app.
///
/// A missing directory is a success, not a failure: removing an app that was never
/// installed is exactly what a person who added a repository and changed their mind is
/// asking for.
///
/// # Errors
/// Returns [`Error::Refused`] when `id` is not a single plain path component — which is
/// what stops a hand-edited catalog turning a removal into a deletion somewhere else —
/// when the app directory is a link rather than a directory, and [`Error::Io`] when the
/// deletion fails.
pub fn remove_installed(paths: &Paths, id: &str) -> Result<Removal> {
    if !is_plain_id(id) {
        return Err(Error::Refused(format!(
            "`{id}` is not an app id, so the launcher will not delete anything for it"
        )));
    }

    let root = paths.install_root();
    let target = paths.app_dir(id);
    // Belt and braces. `app_dir` joins one checked component onto the install root, and
    // this proves it: a removal can only ever reach a direct child of that root.
    if target.parent() != Some(root.as_path()) {
        return Err(Error::Refused(format!(
            "{} is not inside the launcher's install folder, so it will not be removed",
            target.display()
        )));
    }

    let metadata = match std::fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Removal::NothingThere),
        Err(error) => {
            return Err(io_context(
                format!("could not look at {}", target.display()),
                error,
            ));
        }
    };

    // `symlink_metadata` describes the link itself, so this is the one check that can
    // tell a link from a directory. Removing a link would be harmless on most platforms
    // and catastrophic on some, and there is no reason to find out which.
    if metadata.file_type().is_symlink() {
        return Err(Error::Refused(format!(
            "{} is a link, and the launcher will not follow one out of its own folder",
            target.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(Error::Refused(format!(
            "{} is not a folder, so there is nothing here the launcher installed",
            target.display()
        )));
    }

    std::fs::remove_dir_all(&target)
        .map_err(|error| io_context(format!("could not remove {}", target.display()), error))?;
    Ok(Removal::Deleted(target))
}

/// Whether an id is exactly one ordinary path component.
///
/// `Path::components` reads `..`, `.`, a root and a drive letter as their own kinds, so
/// the last check rejects every way of naming something that is not a direct child. The
/// separator check in front of it is what makes that true on **every** platform: on Linux
/// a backslash is an ordinary file-name character, so `a\b` is one component there and
/// two on Windows, and a rule that held on one and not the other is exactly the sort of
/// difference this file is meant to have none of.
///
/// An id with a surrounding space fails the round trip and is rejected too: no catalog the
/// launcher writes produces one, and a name it cannot write is not a name it should delete.
fn is_plain_id(id: &str) -> bool {
    if id.is_empty() || id.trim() != id || id.contains(['/', '\\']) {
        return false;
    }
    let mut components = Path::new(id).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn nothing_installed_is_nothing_to_remove() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        assert_eq!(remove_installed(&paths, "vncli").unwrap(), Removal::NothingThere);
    }

    #[test]
    fn the_launchers_own_copy_is_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        let app = paths.app_dir("vncli");
        std::fs::create_dir_all(app.join("0.1.0")).unwrap();
        std::fs::write(app.join("0.1.0").join("vn"), b"binary").unwrap();

        match remove_installed(&paths, "vncli").unwrap() {
            Removal::Deleted(path) => assert_eq!(path, app),
            other => panic!("expected Deleted, got {other:?}"),
        }
        assert!(!app.exists(), "the app folder should be gone");
        // Its parent — the install root — is the launcher's, and is left in place.
        assert!(paths.install_root().is_dir());
    }

    #[test]
    fn an_id_that_could_name_somewhere_else_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        for id in ["..", ".", "a/b", "a\\b", "C:\\Windows", "", " vncli", "vncli "] {
            let outcome = remove_installed(&paths, id);
            assert!(
                matches!(outcome, Err(Error::Refused(_))),
                "`{id}` should have been refused, got {outcome:?}"
            );
        }
    }

    #[test]
    fn a_file_where_the_app_folder_should_be_is_refused_rather_than_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        let app = paths.app_dir("vncli");
        std::fs::create_dir_all(app.parent().unwrap()).unwrap();
        std::fs::write(&app, b"not a folder").unwrap();

        assert!(matches!(
            remove_installed(&paths, "vncli"),
            Err(Error::Refused(_))
        ));
        assert!(app.exists(), "a removal must not delete a file it did not create");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_refused_rather_than_followed() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        let elsewhere = temp.path().join("somebody-elses-work");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("important.txt"), b"keep me").unwrap();

        std::fs::create_dir_all(paths.install_root()).unwrap();
        std::os::unix::fs::symlink(&elsewhere, paths.app_dir("vncli")).unwrap();

        assert!(matches!(
            remove_installed(&paths, "vncli"),
            Err(Error::Refused(_))
        ));
        assert!(
            elsewhere.join("important.txt").exists(),
            "the link must not be followed"
        );
    }

    #[test]
    fn an_id_is_one_plain_component_or_it_is_not_an_id() {
        assert!(is_plain_id("vncli"));
        assert!(is_plain_id("vn-finance"));
        assert!(is_plain_id("vn.finance"));
        assert!(!is_plain_id(""));
        assert!(!is_plain_id(".."));
        assert!(!is_plain_id("../vncli"));
        assert!(!is_plain_id("/vncli"));
    }
}
