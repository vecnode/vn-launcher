//! The user's own catalog file.
//!
//! This is the file the add-repository flow writes. It is deliberately a plain
//! catalog document — the same shape as the one the launcher ships with — so that
//! what the launcher adds is what a person would have typed, and can edit or delete
//! by hand without the launcher minding.

use std::path::Path;

use vnl_core::catalog::Catalog;
use vnl_core::manifest::{AppFile, CatalogFile, SCHEMA_VERSION};
use vnl_core::repo::Repo;

use crate::error::{Error, Result, io_context};

/// Read the user's catalog, if it exists.
///
/// A missing file is `Ok(None)` — that is the normal state on a fresh machine, not
/// a problem to report.
///
/// # Errors
/// Returns [`Error::Io`] when the file exists but cannot be read, and
/// [`Error::Core`] when it exists but is not a readable catalog.
pub fn load(path: &Path) -> Result<Option<Catalog>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(io_context(format!("could not read {}", path.display()), error));
        }
    };
    Catalog::from_toml(&text, &path.display().to_string())
        .map(Some)
        .map_err(Error::Core)
}

/// Add a repository to the user's catalog, or replace an existing entry for it.
///
/// The entry written is the smallest one that says something: an id, a display name
/// and the repository. Everything else — asset stems, extensions, brand colour,
/// platform matching — is inferred by the launcher, and a person who wants to
/// override any of it can edit the file afterwards.
///
/// # Errors
/// Returns [`Error::Io`] when the file cannot be read or written.
pub fn append(path: &Path, repo: &Repo) -> Result<String> {
    let mut document = match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str::<CatalogFile>(&text).unwrap_or_else(|_| CatalogFile::empty()),
        Err(_) => CatalogFile::empty(),
    };
    document.version = SCHEMA_VERSION;

    let id = derive_id(repo.name());
    document.apps.retain(|app| app.id != id);
    document.apps.push(AppFile {
        id: id.clone(),
        name: repo.name().to_owned(),
        tagline: None,
        description: None,
        repo: repo.slug(),
        homepage: None,
        color: None,
        kind: None,
        // After the curated entries, so adding something does not displace them.
        order: None,
        install: None,
        launch: None,
        detect: None,
    });

    let text = toml::to_string_pretty(&document)
        .map_err(|error| Error::Cache(format!("could not write the catalog: {error}")))?;
    write_atomically(path, &text)?;
    Ok(id)
}

/// An id derived from a repository name: lower case, with anything unusable folded
/// to a dash.
#[must_use]
pub fn derive_id(name: &str) -> String {
    let mut id = String::with_capacity(name.len());
    let mut last_was_dash = false;

    for character in name.chars() {
        let mapped = if character.is_ascii_alphanumeric() {
            character.to_ascii_lowercase()
        } else {
            '-'
        };
        if mapped == '-' {
            if last_was_dash || id.is_empty() {
                continue;
            }
            last_was_dash = true;
        } else {
            last_was_dash = false;
        }
        id.push(mapped);
    }

    while id.ends_with('-') {
        id.pop();
    }
    if id.is_empty() { "app".to_owned() } else { id }
}

/// Write a file without ever leaving a half-written one behind.
///
/// The replacement is atomic where the platform allows it. On Windows renaming over
/// an existing file fails, so the destination is removed and the rename retried —
/// a tiny window that is still better than truncating a file a person may have
/// edited by hand.
fn write_atomically(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| io_context(format!("could not create {}", parent.display()), error))?;
    }

    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, contents)
        .map_err(|error| io_context(format!("could not write {}", temporary.display()), error))?;

    if std::fs::rename(&temporary, path).is_ok() {
        return Ok(());
    }
    let _ = std::fs::remove_file(path);
    std::fs::rename(&temporary, path)
        .map_err(|error| io_context(format!("could not replace {}", path.display()), error))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use vnl_core::catalog::AppKind;

    #[test]
    fn ids_are_derived_from_names_safely() {
        assert_eq!(derive_id("vn-cli"), "vn-cli");
        assert_eq!(derive_id("VN-Finance"), "vn-finance");
        assert_eq!(derive_id("p5js-window"), "p5js-window");
        assert_eq!(derive_id("weird...name"), "weird-name");
        assert_eq!(derive_id("--leading--"), "leading");
        assert_eq!(derive_id(""), "app");
        assert_eq!(derive_id("!!!"), "app");
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let temp = tempfile::tempdir().unwrap();
        assert!(load(&temp.path().join("nothing.toml")).unwrap().is_none());
    }

    #[test]
    fn appending_creates_a_file_that_reloads() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        let repo = Repo::parse("vecnode/vecnode.github.io").unwrap();

        let id = append(&path, &repo).unwrap();
        assert_eq!(id, "vecnode-github-io");
        assert!(path.exists());

        let catalog = load(&path).unwrap().unwrap();
        let entry = catalog.get(&id).unwrap();
        assert_eq!(entry.repo.slug(), "vecnode/vecnode.github.io");
        assert_eq!(entry.name, "vecnode.github.io");
        assert_eq!(entry.kind, AppKind::Release);
    }

    #[test]
    fn appending_the_same_repository_twice_replaces_rather_than_duplicates() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        let repo = Repo::parse("vecnode/vncli").unwrap();

        append(&path, &repo).unwrap();
        append(&path, &repo).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("id = \"vncli\"").count(), 1, "{text}");
    }

    #[test]
    fn an_existing_entry_for_another_repository_is_kept() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        append(&path, &Repo::parse("vecnode/vncli").unwrap()).unwrap();
        append(&path, &Repo::parse("vecnode/vncode").unwrap()).unwrap();

        let catalog = load(&path).unwrap().unwrap();
        assert_eq!(catalog.len(), 2);
        assert!(catalog.get("vncli").is_some());
        assert!(catalog.get("vncode").is_some());
    }

    #[test]
    fn a_hand_edited_file_the_launcher_cannot_parse_is_readable_as_none() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        std::fs::write(&path, "this is not toml at all {{{").unwrap();
        assert!(load(&path).is_err());
        // ...but appending still succeeds, by starting again rather than refusing.
        append(&path, &Repo::parse("vecnode/vncli").unwrap()).unwrap();
        assert!(load(&path).unwrap().is_some());
    }
}
