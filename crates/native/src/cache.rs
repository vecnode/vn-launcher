//! The release-feed cache.
//!
//! The launcher reads GitHub without an account, which means it is a guest: the
//! unauthenticated allowance is 60 requests an hour per address, shared with
//! everything else on the machine. A launcher that re-reads four repositories every
//! time a window is resized would exhaust that and then fail, so a feed read is
//! cached on disk and reused until it is stale.
//!
//! The cache is a convenience, never a source of truth: a missing, unreadable or
//! corrupt file simply means "nothing cached", never an error the person must
//! resolve.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Result, io_context};

/// How long a cached feed is trusted.
pub const DEFAULT_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// The cache file's own schema version.
const CACHE_VERSION: u32 = 1;

/// One cached response.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    /// When it was stored, in seconds since the Unix epoch.
    fetched_at: u64,
    /// The response body, verbatim.
    body: String,
}

/// The cache file as written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct File {
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, Entry>,
}

/// Responses kept on disk between runs.
#[derive(Debug, Clone)]
pub struct FeedCache {
    path: PathBuf,
    ttl: Duration,
    entries: BTreeMap<String, Entry>,
    dirty: bool,
}

impl FeedCache {
    /// Open a cache at `path`, keeping entries for `ttl`.
    ///
    /// A file that cannot be read is treated as an empty cache.
    #[must_use]
    pub fn load(path: impl Into<PathBuf>, ttl: Duration) -> Self {
        let path = path.into();
        let entries = read_entries(&path);
        Self {
            path,
            ttl,
            entries,
            dirty: false,
        }
    }

    /// How long an entry stays fresh.
    #[must_use]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Change how long entries stay fresh.
    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }

    /// A cached body, if one is present and still fresh.
    ///
    /// An entry is fresh while it is *younger* than the TTL, so a TTL of zero means
    /// "never serve from the cache" rather than "serve anything stored this second".
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        let entry = self.entries.get(key)?;
        let age = now_unix().saturating_sub(entry.fetched_at);
        (age < self.ttl.as_secs()).then_some(entry.body.as_str())
    }

    /// Store a body, marking the cache as needing a write.
    pub fn put(&mut self, key: &str, body: &str) {
        self.entries.insert(
            key.to_owned(),
            Entry {
                fetched_at: now_unix(),
                body: body.to_owned(),
            },
        );
        self.dirty = true;
    }

    /// How many responses are held, fresh or not.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forget everything, including anything on disk.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file exists and cannot be removed.
    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        self.dirty = false;
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_context(format!("could not remove {}", self.path.display()), e)),
        }
    }

    /// Write the cache back, if anything changed.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file cannot be written.
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| io_context(format!("could not create {}", parent.display()), e))?;
        }
        let file = File {
            version: CACHE_VERSION,
            entries: self.entries.clone(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|e| crate::Error::Cache(e.to_string()))?;
        std::fs::write(&self.path, text)
            .map_err(|e| io_context(format!("could not write {}", self.path.display()), e))?;
        self.dirty = false;
        Ok(())
    }

    /// The file this cache lives in.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Read the entries from a cache file, treating any problem as "nothing cached".
fn read_entries(path: &Path) -> BTreeMap<String, Entry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    match serde_json::from_str::<File>(&text) {
        Ok(file) if file.version == CACHE_VERSION => file.entries,
        Ok(file) => {
            log::debug!(
                "ignoring feed cache {}: schema version {} is not {CACHE_VERSION}",
                path.display(),
                file.version
            );
            BTreeMap::new()
        }
        Err(e) => {
            log::debug!("ignoring unreadable feed cache {}: {e}", path.display());
            BTreeMap::new()
        }
    }
}

/// Seconds since the Unix epoch, or 0 if the clock is before it.
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn a_stored_body_comes_back() {
        let temp = tempfile::tempdir().unwrap();
        let mut cache = FeedCache::load(temp.path().join("feeds.json"), DEFAULT_TTL);
        assert!(cache.get("k").is_none());
        cache.put("k", "{\"hello\":true}");
        assert_eq!(cache.get("k"), Some("{\"hello\":true}"));
    }

    #[test]
    fn a_body_survives_a_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("feeds.json");

        let mut cache = FeedCache::load(&path, DEFAULT_TTL);
        cache.put("k", "body");
        cache.save().unwrap();

        let reopened = FeedCache::load(&path, DEFAULT_TTL);
        assert_eq!(reopened.get("k"), Some("body"));
    }

    #[test]
    fn an_entry_older_than_the_ttl_is_not_served() {
        let temp = tempfile::tempdir().unwrap();
        let mut cache = FeedCache::load(temp.path().join("feeds.json"), Duration::ZERO);
        cache.put("k", "body");
        // A zero TTL makes the entry stale immediately.
        assert!(cache.get("k").is_none());
        assert_eq!(cache.len(), 1, "the entry is stale, not gone");
    }

    #[test]
    fn a_corrupt_cache_is_empty_rather_than_fatal() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("feeds.json");
        std::fs::write(&path, "this is not json").unwrap();
        let cache = FeedCache::load(&path, DEFAULT_TTL);
        assert!(cache.is_empty());
    }

    #[test]
    fn a_cache_from_a_newer_schema_is_ignored() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("feeds.json");
        std::fs::write(
            &path,
            r#"{"version":99,"entries":{"k":{"fetched_at":1,"body":"x"}}}"#,
        )
        .unwrap();
        assert!(FeedCache::load(&path, DEFAULT_TTL).is_empty());
    }

    #[test]
    fn clearing_removes_the_file_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("feeds.json");
        let mut cache = FeedCache::load(&path, DEFAULT_TTL);
        cache.put("k", "body");
        cache.save().unwrap();
        assert!(path.exists());
        cache.clear().unwrap();
        assert!(!path.exists());
        cache.clear().unwrap();
    }

    #[test]
    fn saving_an_unchanged_cache_touches_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("feeds.json");
        let mut cache = FeedCache::load(&path, DEFAULT_TTL);
        cache.save().unwrap();
        assert!(!path.exists(), "an untouched cache must not be written");
    }
}
