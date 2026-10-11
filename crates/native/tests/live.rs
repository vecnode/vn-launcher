//! Tests that talk to the real GitHub API.
//!
//! Ignored by default, so CI never depends on the network, on GitHub being up, or on
//! an anonymous rate-limit allowance being left. Run them deliberately:
//!
//! ```text
//! cargo test -p vnl-native --test live -- --ignored --nocapture
//! ```
//!
//! They are worth running by hand after any change to the HTTP layer or to the asset
//! matching, because they are the only tests that use the payloads GitHub actually
//! sends rather than a fixture.
//!
//! This file is its own crate, so the workspace's no-panic rules — which exist for
//! shipped code — are stood down here, where a panic is how a test fails.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use vnl_core::status::Availability;
use vnl_core::{Catalog, Target};
use vnl_native::cache::FeedCache;
use vnl_native::http::Client;
use vnl_native::{Feed, Paths};

/// A feed reading through a cache directory that is thrown away afterwards.
fn feed() -> (Feed, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let cache = FeedCache::load(directory.path().join("feeds.json"), Duration::from_secs(60));
    (Feed::new(Client::new(), cache), directory)
}

#[test]
#[ignore = "requires network access to api.github.com"]
fn the_live_feed_reads_every_shipped_app() {
    let (mut feed, _directory) = feed();
    let catalog = Catalog::builtin().expect("the built-in catalog");
    let target = Target::current();

    for entry in catalog.entries() {
        let availability = feed.availability(entry, &target);
        println!("{}: {availability:?}", entry.id);

        match &availability {
            Availability::Failed(reason) => {
                panic!("{} could not be read: {reason}", entry.id);
            }
            Availability::NoReleases => {
                // Legitimate: `vn-finance` and `vn-mediabus` publish nothing yet.
                println!("   (no releases published)");
            }
            Availability::Release(info) => {
                println!(
                    "   tag={} asset={:?}",
                    info.tag,
                    info.asset.chosen().map(|asset| asset.name.as_str())
                );
            }
            other => println!("   {other:?}"),
        }
    }
}

#[test]
#[ignore = "requires network access to api.github.com"]
fn the_live_repository_check_tells_missing_from_empty() {
    let client = Client::new();

    let present = client.get_json(
        &vnl_core::Repo::parse("vecnode/vncli")
            .expect("a repository")
            .api_repo_url(),
        vnl_native::http::MAX_PROBE_BYTES,
    );
    match present {
        Ok((status, _)) => assert_eq!(status, 200, "vecnode/vncli should exist"),
        Err(error) => panic!("the repository endpoint could not be reached: {error}"),
    }

    let missing = client.get_json(
        &vnl_core::Repo::parse("vecnode/this-does-not-exist-4f2a")
            .expect("a repository")
            .api_repo_url(),
        vnl_native::http::MAX_PROBE_BYTES,
    );
    match missing {
        Ok((status, _)) => assert_eq!(status, 404, "a missing repository should 404"),
        Err(error) => panic!("the repository endpoint could not be reached: {error}"),
    }
}

#[test]
#[ignore = "requires network access to api.github.com"]
fn the_paths_helper_resolves_a_real_home_directory() {
    let paths = Paths::discover().expect("a configuration directory");
    println!("config: {}", paths.config_dir().display());
    println!("cache:  {}", paths.cache_dir().display());
    println!("data:   {}", paths.data_dir().display());
}
