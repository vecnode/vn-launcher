//! The user's own catalog file.
//!
//! This is the file the add-repository flow writes, and the one Remove writes a
//! tombstone into. It is deliberately a plain catalog document — the same shape as the
//! one the launcher ships with — so that what the launcher adds is what a person would
//! have typed, and can edit or delete by hand without the launcher minding.

use std::path::Path;

use vnl_core::catalog::Catalog;
use vnl_core::manifest::{AppFile, CatalogFile, SCHEMA_VERSION};
use vnl_core::repo::Repo;

use crate::error::{Error, Result, io_context};

/// What the launcher writes at the top of the file, so that a person who opens it sees
/// what the two halves of the document mean without reading this crate.
const FILE_HEADER: &str = "\
# vn-launcher's own catalog. An entry here is merged over the one the launcher ships
# with, by id — the same id replaces it. `removed` lists apps you took out of the
# launcher, so that removing a built-in app survives a restart; adding a repository
# back takes its id out of the list again.
";

/// A user's catalog as read from disk.
///
/// Two facts, because the merged catalog cannot express both on its own: the entries to
/// merge over the built-in catalog, and the ids to drop *afterwards*. Dropping them
/// afterwards is what makes a removal win over the shipped entry and over an override
/// of it.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    /// The entries, ready to be merged.
    pub catalog: Catalog,
    /// Ids the person removed, normalised.
    pub removed: Vec<String>,
}

/// Read the user's catalog, if it exists.
///
/// A missing file is `Ok(None)` — that is the normal state on a fresh machine, not
/// a problem to report.
///
/// # Errors
/// Returns [`Error::Io`] when the file exists but cannot be read, and
/// [`Error::Core`] when it exists but is not a readable catalog.
pub fn load(path: &Path) -> Result<Option<Overlay>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(io_context(format!("could not read {}", path.display()), error));
        }
    };

    let source = path.display().to_string();
    let document: CatalogFile =
        toml::from_str(&text).map_err(|e| Error::Core(vnl_core::Error::Catalog(format!("{source}: {e}"))))?;
    let removed = normalise(&document.removed);
    let catalog = Catalog::from_document(document, &source)?;
    Ok(Some(Overlay { catalog, removed }))
}

/// Add a repository to the user's catalog, or replace an existing entry for it.
///
/// The entry written is the smallest one that says something: an id, a display name,
/// the repository, and — when one was named — the folder on this machine where it already
/// lives. Everything else — asset stems, extensions, brand colour, platform matching — is
/// inferred by the launcher, and a person who wants to override any of it can edit the
/// file afterwards.
///
/// Adding a repository also takes its id **out of the removed list**: adding something
/// back is the obvious undo for having taken it away.
///
/// `local` is the second half of the add-repository flow: the repository says where the
/// code comes from, the folder says where it already is. Passing `None` leaves an existing
/// folder alone rather than clearing it, so re-adding a repository does not forget it.
///
/// # Errors
/// Returns [`Error::Io`] when the file cannot be read or written.
pub fn append(path: &Path, repo: &Repo, local: Option<&str>) -> Result<String> {
    let mut document = read_document(path);
    document.version = SCHEMA_VERSION;

    let id = derive_id(repo.name());
    let kept = document
        .apps
        .iter()
        .find(|app| app.id == id)
        .and_then(|app| app.local.clone());
    let local = local
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(str::to_owned)
        .or(kept);

    document.apps.retain(|app| app.id != id);
    document.removed.retain(|removed| removed != &id);
    document.apps.push(AppFile {
        id: id.clone(),
        name: repo.name().to_owned(),
        tagline: None,
        description: None,
        repo: repo.slug(),
        homepage: None,
        color: None,
        kind: None,
        local,
        // After the curated entries, so adding something does not displace them.
        order: None,
        install: None,
        launch: None,
        detect: None,
    });

    write_document(path, &document)?;
    Ok(id)
}

/// Take an app out of the launcher.
///
/// Two things are written, and both are needed. The entry is dropped from this file, so
/// an app the person added themselves disappears; the id is added to `removed`, so an
/// app that came from the catalog this build ships with — which is compiled into the
/// binary and therefore cannot be deleted — stays gone at the next start too.
///
/// Nothing outside this file is touched here. Deleting what the launcher installed is
/// `crate::uninstall`'s job, and it only ever reaches into the launcher's own install
/// root.
///
/// # Errors
/// Returns [`Error::Refused`] for an empty id, and [`Error::Io`] when the file cannot be
/// written.
pub fn remove(path: &Path, id: &str) -> Result<()> {
    let id = id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return Err(Error::Refused(
            "an app id is needed to remove anything".to_owned(),
        ));
    }

    let mut document = read_document(path);
    document.version = SCHEMA_VERSION;
    document.apps.retain(|app| app.id != id);
    if !document.removed.iter().any(|removed| removed == &id) {
        document.removed.push(id);
        document.removed.sort();
    }

    write_document(path, &document)
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

/// Read a document, treating an unreadable one as empty.
///
/// Writing has to work on a file a person may have broken by hand: the alternative is a
/// launcher that refuses to add anything until a text file it cannot parse is fixed.
/// Reading it properly, in [`load`], is where that is reported instead.
fn read_document(path: &Path) -> CatalogFile {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str::<CatalogFile>(&text).ok())
        .unwrap_or_else(CatalogFile::empty)
}

/// Write a document back, with the header that explains the file.
fn write_document(path: &Path, document: &CatalogFile) -> Result<()> {
    let body = toml::to_string_pretty(document)
        .map_err(|error| Error::Cache(format!("could not write the catalog: {error}")))?;
    write_atomically(path, &format!("{FILE_HEADER}\n{body}"))
}

/// Trimmed, lower-cased, non-empty ids, in the order they were written.
fn normalise(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.trim().to_ascii_lowercase();
        if id.is_empty() || out.contains(&id) {
            continue;
        }
        out.push(id);
    }
    out
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

        let id = append(&path, &repo, None).unwrap();
        assert_eq!(id, "vecnode-github-io");
        assert!(path.exists());

        let overlay = load(&path).unwrap().unwrap();
        let entry = overlay.catalog.get(&id).unwrap();
        assert_eq!(entry.repo.slug(), "vecnode/vecnode.github.io");
        assert_eq!(entry.name, "vecnode.github.io");
        assert_eq!(entry.kind, AppKind::Release);
        assert!(overlay.removed.is_empty());
    }

    #[test]
    fn the_file_the_launcher_writes_explains_itself() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        append(&path, &Repo::parse("vecnode/vncli").unwrap(), None).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with('#'),
            "the file should open with a comment:\n{text}"
        );
        assert!(text.contains("`removed`"), "{text}");
        // A header must not stop the document being read back as TOML.
        assert!(toml::from_str::<CatalogFile>(&text).is_ok());
    }

    #[test]
    fn appending_the_same_repository_twice_replaces_rather_than_duplicates() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        let repo = Repo::parse("vecnode/vncli").unwrap();

        append(&path, &repo, None).unwrap();
        append(&path, &repo, None).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("id = \"vncli\"").count(), 1, "{text}");
    }

    #[test]
    fn an_existing_entry_for_another_repository_is_kept() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        append(&path, &Repo::parse("vecnode/vncli").unwrap(), None).unwrap();
        append(&path, &Repo::parse("vecnode/vncode").unwrap(), None).unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.catalog.len(), 2);
        assert!(overlay.catalog.get("vncli").is_some());
        assert!(overlay.catalog.get("vncode").is_some());
    }

    #[test]
    fn a_named_folder_is_written_and_survives_a_second_add() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        let repo = Repo::parse("vecnode/vncli").unwrap();

        let id = append(&path, &repo, Some("C:\\src\\vncli")).unwrap();
        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(
            overlay.catalog.get(&id).unwrap().local.as_deref(),
            Some("C:\\src\\vncli")
        );
        assert!(overlay.catalog.get(&id).unwrap().has_local_folder());

        // Adding the same repository again without naming a folder must not forget one.
        append(&path, &repo, None).unwrap();
        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(
            overlay.catalog.get(&id).unwrap().local.as_deref(),
            Some("C:\\src\\vncli"),
            "re-adding a repository should not lose where it lives"
        );

        // Naming a different one replaces it.
        append(&path, &repo, Some("D:\\elsewhere")).unwrap();
        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(
            overlay.catalog.get(&id).unwrap().local.as_deref(),
            Some("D:\\elsewhere")
        );
    }

    #[test]
    fn a_blank_folder_is_written_as_nothing_at_all() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        let repo = Repo::parse("vecnode/vncode").unwrap();

        let id = append(&path, &repo, Some("   ")).unwrap();
        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.catalog.get(&id).unwrap().local, None);

        // And it is left out of the file rather than written as an empty string.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("local"), "{text}");
    }

    #[test]
    fn removing_an_app_writes_a_tombstone_that_a_later_read_reports() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");

        remove(&path, "vncli").unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.removed, ["vncli"]);
        // Nothing else was written: a removal of a built-in app is one line, not an
        // entry that shadows it.
        assert!(overlay.catalog.is_empty());
    }

    #[test]
    fn removing_an_app_the_user_added_drops_its_entry_as_well_as_marking_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        append(&path, &Repo::parse("vecnode/vncode").unwrap(), None).unwrap();

        remove(&path, "vncode").unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert!(overlay.catalog.get("vncode").is_none());
        assert_eq!(overlay.removed, ["vncode"]);
    }

    #[test]
    fn removing_twice_leaves_one_tombstone() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");

        remove(&path, "vncli").unwrap();
        remove(&path, "VNCLI").unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.removed, ["vncli"]);
    }

    #[test]
    fn adding_a_repository_back_takes_its_id_out_of_the_removed_list() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        remove(&path, "vncli").unwrap();

        append(&path, &Repo::parse("vecnode/vncli").unwrap(), None).unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert!(
            overlay.removed.is_empty(),
            "adding the repository back must undo the removal: {:?}",
            overlay.removed
        );
        assert!(overlay.catalog.get("vncli").is_some());
    }

    #[test]
    fn removing_and_then_adding_something_else_keeps_the_tombstone() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        remove(&path, "vncli").unwrap();

        append(&path, &Repo::parse("vecnode/vncode").unwrap(), None).unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.removed, ["vncli"]);
        assert!(overlay.catalog.get("vncode").is_some());
    }

    #[test]
    fn an_empty_id_is_refused_rather_than_writing_a_blank_tombstone() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        assert!(matches!(remove(&path, "   "), Err(Error::Refused(_))));
        assert!(!path.exists());
    }

    #[test]
    fn a_hand_edited_file_the_launcher_cannot_parse_is_readable_as_none() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        std::fs::write(&path, "this is not toml at all {{{").unwrap();
        assert!(load(&path).is_err());
        // ...but appending still succeeds, by starting again rather than refusing.
        append(&path, &Repo::parse("vecnode/vncli").unwrap(), None).unwrap();
        assert!(load(&path).unwrap().is_some());
    }

    #[test]
    fn a_hand_written_removed_list_is_normalised() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("apps.toml");
        std::fs::write(
            &path,
            "version = 1\nremoved = [\"  VNCLI \", \"\", \"vncli\", \"gone\"]\n",
        )
        .unwrap();

        let overlay = load(&path).unwrap().unwrap();
        assert_eq!(overlay.removed, ["vncli", "gone"]);
    }
}
