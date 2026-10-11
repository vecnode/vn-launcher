//! The on-disk shape of a catalog file.
//!
//! These types read *and write* TOML, because the add-repository flow appends an
//! entry to the user's own catalog rather than keeping a private database beside it.
//! The file a person edits by hand is the same file the launcher edits.
//!
//! They are deliberately permissive — every field but `id`, `name` and `repo` is
//! optional — so a hand-written entry that fills in only what it needs still loads.
//! Turning a [`CatalogFile`] into the validated [`crate::catalog::Catalog`] is
//! `catalog.rs`'s job, and that is where anything wrong is reported with the id
//! that caused it.
//!
//! Field order matters: TOML requires a table's scalar values to precede its
//! sub-tables, and the serialiser writes fields in declaration order. `order`
//! therefore sits with the scalars, above `install`, `launch` and `detect`.

use serde::{Deserialize, Serialize};

/// The current catalog schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// One catalog document: the built-in one, or a user's.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogFile {
    /// The schema version the document was written against.
    #[serde(default = "default_schema_version")]
    pub version: u32,
    /// The apps it declares.
    #[serde(default, rename = "app")]
    pub apps: Vec<AppFile>,
}

impl CatalogFile {
    /// An empty document at the current schema version.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: SCHEMA_VERSION,
            apps: Vec::new(),
        }
    }
}

/// One app in a catalog document.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppFile {
    /// Stable machine name.
    pub id: String,
    /// Display name.
    pub name: String,
    /// One line describing what it is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    /// A paragraph for the detail view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The GitHub repository, `owner/name` or any URL form.
    pub repo: String,
    /// Where to send someone who wants the project itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// The brand colour, `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// `release` (installs from GitHub Releases) or `link` (opens a page).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// A folder on this machine where the app already lives — a clone, a build, or
    /// wherever a person keeps it. Set by the add-repository flow, which asks for the
    /// repository first and the local place second.
    ///
    /// It sits with the scalars, above the tables: TOML requires a table's plain values
    /// to precede its sub-tables, and the serialiser writes fields in declaration order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    /// Display position; lower sorts first. Ties fall back to the id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    /// How to install it, once installation exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<InstallFile>,
    /// How to start it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchFile>,
    /// How to tell whether it is already on this machine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detect: Option<DetectFile>,
}

/// The `[app.install]` table.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InstallFile {
    /// File-name prefixes this app's release assets use.
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "asset_stems")]
    pub stems: Vec<String>,
    /// Archive extensions to accept, in preference order.
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "prefer")]
    pub extensions: Vec<String>,
    /// The executable's name, without an extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    /// Where to send someone when the launcher cannot install it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}

/// The `[app.launch]` table: an argv template per operating system.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LaunchFile {
    /// The argument vector on Windows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<String>,
    /// The argument vector on macOS.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub macos: Vec<String>,
    /// The argument vector on Linux.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linux: Vec<String>,
}

/// The `[app.detect]` table.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DetectFile {
    /// Candidate paths, which may use the `{install}` placeholder.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// A command to look for on `PATH`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_document_round_trips_through_toml() {
        let original = CatalogFile {
            version: SCHEMA_VERSION,
            apps: vec![AppFile {
                id: "my-tool".to_owned(),
                name: "My Tool".to_owned(),
                tagline: Some("Does a thing".to_owned()),
                description: None,
                repo: "someone/my-tool".to_owned(),
                homepage: None,
                color: Some("#3b82f6".to_owned()),
                kind: None,
                local: None,
                order: None,
                install: Some(InstallFile {
                    stems: vec!["my-tool".to_owned()],
                    extensions: vec!["zip".to_owned()],
                    binary: Some("mytool".to_owned()),
                    docs: None,
                }),
                launch: Some(LaunchFile {
                    windows: vec!["{bin}".to_owned()],
                    macos: vec![],
                    linux: vec!["{bin}".to_owned()],
                }),
                detect: Some(DetectFile {
                    paths: vec!["{install}/mytool".to_owned()],
                    command: Some("mytool".to_owned()),
                }),
            }],
        };

        let text = toml::to_string_pretty(&original).unwrap();
        // Scalar values must precede sub-tables, or TOML cannot be re-read.
        let order_position = text.find("order =").unwrap_or(0);
        let install_position = text.find("[app.install]").unwrap();
        assert!(
            order_position < install_position || order_position == 0,
            "`order` must be written before `[app.install]`:\n{text}"
        );

        let reparsed: CatalogFile = toml::from_str(&text).unwrap();
        assert_eq!(reparsed.apps.len(), 1);
        let app = &reparsed.apps[0];
        assert_eq!(app.id, "my-tool");
        assert_eq!(app.repo, "someone/my-tool");
        assert_eq!(app.color.as_deref(), Some("#3b82f6"));
        assert_eq!(app.install.as_ref().unwrap().binary.as_deref(), Some("mytool"));
        assert_eq!(app.launch.as_ref().unwrap().linux, ["{bin}"]);
        assert_eq!(app.detect.as_ref().unwrap().command.as_deref(), Some("mytool"));
    }

    #[test]
    fn absent_optional_fields_are_left_out_entirely() {
        let document = CatalogFile {
            version: SCHEMA_VERSION,
            apps: vec![AppFile {
                id: "a".to_owned(),
                name: "A".to_owned(),
                tagline: None,
                description: None,
                repo: "owner/a".to_owned(),
                homepage: None,
                color: None,
                kind: None,
                local: None,
                order: None,
                install: None,
                launch: None,
                detect: None,
            }],
        };
        let text = toml::to_string_pretty(&document).unwrap();
        assert!(!text.contains("tagline"), "{text}");
        assert!(!text.contains("[app.install]"), "{text}");
        assert!(text.contains("id = \"a\""), "{text}");
    }

    #[test]
    fn the_written_form_reloads_as_a_catalog() {
        use crate::catalog::Catalog;

        let document = CatalogFile {
            version: SCHEMA_VERSION,
            apps: vec![AppFile {
                id: "vecnode-site".to_owned(),
                name: "vecnode.github.io".to_owned(),
                tagline: None,
                description: None,
                repo: "vecnode/vecnode.github.io".to_owned(),
                homepage: None,
                color: None,
                kind: Some("link".to_owned()),
                local: None,
                order: None,
                install: None,
                launch: None,
                detect: None,
            }],
        };
        let text = toml::to_string_pretty(&document).unwrap();
        let catalog = Catalog::from_toml(&text, "written by the launcher").unwrap();
        let entry = catalog.get("vecnode-site").unwrap();
        assert_eq!(entry.kind, crate::catalog::AppKind::Link);
        assert_eq!(entry.repo.slug(), "vecnode/vecnode.github.io");
    }
}
