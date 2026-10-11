//! Where the launcher keeps its files.
//!
//! One place decides this, so the config file, the feed cache and the install root
//! can never disagree about where "here" is:
//!
//! | What | Windows | macOS | Linux |
//! |---|---|---|---|
//! | config | `%APPDATA%\vn-launcher` | `~/Library/Application Support/vn-launcher` | `~/.config/vn-launcher` |
//! | cache | `%LOCALAPPDATA%\vn-launcher\cache` | `~/Library/Caches/vn-launcher` | `~/.cache/vn-launcher` |
//! | data | `%LOCALAPPDATA%\vn-launcher\data` | `~/Library/Application Support/vn-launcher` | `~/.local/share/vn-launcher` |

use std::path::{Path, PathBuf};

use crate::error::{Error, Result, io_context};

/// The directory name used under every platform's own location.
const APP_DIR: &str = "vn-launcher";

/// The user's catalog, merged over the built-in one.
pub const USER_CATALOG_FILE: &str = "apps.toml";

/// The cached release feeds.
const FEED_CACHE_FILE: &str = "feeds.json";

/// The launcher's own directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    data_dir: PathBuf,
}

impl Paths {
    /// Where this user's launcher files belong.
    ///
    /// # Errors
    /// Returns [`Error::NoConfigDir`] when the operating system reports no home or
    /// application-data directory, which happens only in unusual service contexts.
    pub fn discover() -> Result<Self> {
        let config_dir = dirs::config_dir().ok_or(Error::NoConfigDir)?.join(APP_DIR);
        let cache_dir = dirs::cache_dir().ok_or(Error::NoConfigDir)?.join(APP_DIR);
        let data_dir = dirs::data_dir().ok_or(Error::NoConfigDir)?.join(APP_DIR);
        Ok(Self {
            config_dir,
            cache_dir,
            data_dir,
        })
    }

    /// Build a layout under an explicit root, for tests.
    #[must_use]
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().join(APP_DIR);
        Self {
            config_dir: root.clone(),
            cache_dir: root.join("cache"),
            data_dir: root.join("data"),
        }
    }

    /// The configuration directory.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// The cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// The data directory, which is where installed apps eventually live.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The user's catalog file.
    #[must_use]
    pub fn user_catalog(&self) -> PathBuf {
        self.config_dir.join(USER_CATALOG_FILE)
    }

    /// The cached release feeds.
    #[must_use]
    pub fn feed_cache(&self) -> PathBuf {
        self.cache_dir.join(FEED_CACHE_FILE)
    }

    /// Where apps will be installed, once installation exists.
    #[must_use]
    pub fn install_root(&self) -> PathBuf {
        self.data_dir.join("apps")
    }

    /// Where an individual app will be installed, once installation exists.
    #[must_use]
    pub fn app_dir(&self, id: &str) -> PathBuf {
        self.install_root().join(id)
    }

    /// Create the directories the launcher writes to.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if a directory cannot be created.
    pub fn ensure_dirs(&self) -> Result<()> {
        for directory in [&self.config_dir, &self.cache_dir, &self.data_dir] {
            std::fs::create_dir_all(directory)
                .map_err(|e| io_context(format!("could not create {}", directory.display()), e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn an_explicit_root_keeps_everything_beneath_it() {
        let paths = Paths::under("/tmp/example");
        assert!(paths.config_dir().starts_with("/tmp/example"));
        assert!(paths.cache_dir().starts_with("/tmp/example"));
        assert!(paths.data_dir().starts_with("/tmp/example"));
        assert_eq!(paths.user_catalog(), paths.config_dir().join("apps.toml"));
        assert!(paths.install_root().starts_with(paths.data_dir()));
        assert!(paths.app_dir("vncli").ends_with("apps/vncli"));
    }

    #[test]
    fn the_real_layout_is_populated_on_this_machine() {
        let paths = Paths::discover().unwrap();
        assert!(paths.config_dir().ends_with("vn-launcher"));
        assert!(paths.cache_dir().ends_with("vn-launcher"));
    }

    #[test]
    fn creating_the_directories_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let paths = Paths::under(temp.path());
        paths.ensure_dirs().unwrap();
        paths.ensure_dirs().unwrap();
        assert!(paths.cache_dir().is_dir());
    }
}
