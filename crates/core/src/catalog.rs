//! The catalog: which apps this launcher knows about, and how to treat each one.
//!
//! The catalog is data, not code. `assets/catalog/apps.toml` is compiled in as the
//! shipped default, and a user catalog at the platform's config directory is merged
//! on top of it, entry by entry, by `id`. Adding a repository of your own is
//! therefore a line in a text file, not a rebuild — which is the whole point.

use std::collections::BTreeMap;

use crate::asset::is_known_extension;
use crate::color::Color;
use crate::error::{Error, Result};
use crate::manifest::{AppFile, CatalogFile, DetectFile, InstallFile, LaunchFile, SCHEMA_VERSION};
use crate::platform::Os;
use crate::repo::Repo;

/// The shipped catalog, compiled into the binary.
pub const BUILTIN_TOML: &str = include_str!("../../../assets/catalog/apps.toml");

/// Where an entry with no explicit `order` sorts.
///
/// After every curated entry, so an app pasted into a user's catalog appears at the
/// end of the grid rather than jumping to the front of it. Ties fall back to the id,
/// which keeps the order stable between runs.
const UNSORTED_ORDER: i32 = 1_000;

/// How far apart two tile colours must be to count as different.
///
/// Compared as a distance in RGB rather than by equality: `#3b82f6` and `#3c83f7` are
/// different values and the same colour to a person, and a grid where two tiles look
/// identical has failed even though the bytes differ.
///
/// 30 is chosen from what the separation buys rather than from taste. Adjacent tiles are
/// 44 points apart on screen, so two colours are only ever compared across a gap; at this
/// distance they are plainly different, and at 10 they would not be. The number matters
/// because capacity falls steeply as it rises — the same palette holds roughly 37 colours
/// at 45 and over a hundred here, and a launcher someone adds repositories to needs more
/// than 37.
const MIN_COLOR_DISTANCE: f32 = 28.0;

/// The lightness and saturation bands generated colours are drawn from.
///
/// Varying hue alone is not enough, and the first attempt at this proved it twice over:
/// every candidate then sits on a single closed curve through the colour space, whose
/// whole length fits only about a dozen colours at the distance above. Crossing hue with
/// these bands spreads candidates through the volume instead.
///
/// The bands are spaced by a tenth of a lightness step deliberately: two colours in
/// adjacent bands are then about [`MIN_COLOR_DISTANCE`] apart, which is what makes the
/// crossing worth anything.
const LIGHTNESS_BANDS: [f32; 4] = [0.38, 0.50, 0.62, 0.74];
const SATURATION_BANDS: [f32; 2] = [0.55, 0.75];

/// How many hues the generated sequence uses.
///
/// Hue is stepped in even slices rather than by the golden angle. The golden angle looks
/// better on paper and is worse in practice: deriving the band from the same counter
/// makes the two resonate, and hue then advances by a constant small amount each time a
/// band combination comes round again — so the sequence revisits the same handful of
/// colours instead of covering the space. Even slices crossed with the bands give a pool
/// of 24 × 4 × 2 colours.
const HUE_STEPS: usize = 24;

/// How many candidates to try before deciding the pool is exhausted.
///
/// The pool holds 24 × 4 × 2 colours, so this walks it several times over. Four times is
/// enough that a failure means the pool really is full rather than that the walk was
/// unlucky in the order it visited.
const POOL_ATTEMPTS: usize = 768;

/// What a catalog entry describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKind {
    /// Obtained from the repository's GitHub Releases.
    Release,
    /// Opened as a page; there is nothing to install.
    Link,
}

impl AppKind {
    /// The spelling used in a catalog file.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Link => "link",
        }
    }
}

/// How an app is obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallSpec {
    /// File-name prefixes this app's release assets are known to use.
    pub stems: Vec<String>,
    /// Archive extensions to accept, in preference order. Empty means "whatever this
    /// operating system normally ships".
    pub extensions: Vec<String>,
    /// The executable's name without an extension.
    pub binary: Option<String>,
    /// Where to send someone when the launcher cannot install it.
    pub docs: Option<String>,
}

impl InstallSpec {
    /// The binary to look for, falling back to the app id.
    #[must_use]
    pub fn binary_or(&self, id: &str) -> String {
        self.binary.clone().unwrap_or_else(|| id.to_owned())
    }
}

/// How an app is started.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchSpec {
    /// The argument vector on Windows.
    pub windows: Vec<String>,
    /// The argument vector on macOS.
    pub macos: Vec<String>,
    /// The argument vector on Linux.
    pub linux: Vec<String>,
}

impl LaunchSpec {
    /// The argument vector for an operating system.
    #[must_use]
    pub fn for_os(&self, os: Os) -> &[String] {
        match os {
            Os::Windows => &self.windows,
            Os::MacOs => &self.macos,
            Os::Linux | Os::Other => &self.linux,
        }
    }

    /// Whether any operating system has a launch command at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty() && self.macos.is_empty() && self.linux.is_empty()
    }
}

/// How to tell whether an app is already on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetectSpec {
    /// Candidate paths; `{install}` is replaced with the launcher's install root for
    /// that app.
    pub paths: Vec<String>,
    /// A command to look for on `PATH`.
    pub command: Option<String>,
}

/// One app, resolved and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    /// Stable machine name, unique within a catalog.
    pub id: String,
    /// Display name.
    pub name: String,
    /// One line describing what it is for.
    pub tagline: String,
    /// A paragraph for the detail view.
    pub description: String,
    /// The GitHub repository.
    pub repo: Repo,
    /// Where to send someone who wants the project itself.
    pub homepage: String,
    /// A folder on this machine where the app already is, when one was named.
    ///
    /// Held as a string rather than a `PathBuf` so the core stays a pure data crate; the
    /// desktop side decides what a path means.
    pub local: Option<String>,
    /// The brand colour used for the app's tile.
    pub color: Color,
    /// What kind of entry this is.
    pub kind: AppKind,
    /// How it is obtained.
    pub install: InstallSpec,
    /// How it is started.
    pub launch: LaunchSpec,
    /// How its presence is detected.
    pub detect: DetectSpec,
    /// Display position; lower sorts first.
    pub order: i32,
}

impl AppEntry {
    /// The repository's Releases page.
    #[must_use]
    pub fn releases_url(&self) -> String {
        self.repo.releases_url()
    }

    /// Where a person should be sent for manual instructions.
    #[must_use]
    pub fn install_docs_url(&self) -> String {
        self.install.docs.clone().unwrap_or_else(|| self.repo.html_url())
    }

    /// Where to send someone for the project itself.
    #[must_use]
    pub fn homepage_or_repo(&self) -> String {
        if self.homepage.is_empty() {
            self.repo.html_url()
        } else {
            self.homepage.clone()
        }
    }

    /// Two characters, for a tile too small to hold the name.
    ///
    /// Split on separators and camel-case, so `vn-finance` gives `VF` and `PhotoCraft`
    /// gives `PC`. A single word takes its **first and last** letters rather than its
    /// first two, because `vncli` and `vncode` both began `VN` — which left two tiles in
    /// the same grid telling the same story and differing only in colour.
    #[must_use]
    pub fn monogram(&self) -> String {
        let words = split_words(&self.name);
        let letters: String = match words.as_slice() {
            [] => self.id.chars().take(2).collect(),
            [only] => match (only.chars().next(), only.chars().next_back()) {
                (Some(first), Some(last)) if first != last => [first, last].iter().collect(),
                (Some(first), _) => first.to_string(),
                _ => self.id.chars().take(2).collect(),
            },
            [first, second, ..] => first.chars().take(1).chain(second.chars().take(1)).collect(),
        };
        letters.to_uppercase()
    }

    /// Whether a folder on this machine was named for this app.
    #[must_use]
    pub fn has_local_folder(&self) -> bool {
        self.local.as_deref().is_some_and(|path| !path.trim().is_empty())
    }
}

/// A set of apps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    entries: Vec<AppEntry>,
}

impl Catalog {
    /// The catalog this build ships with.
    ///
    /// # Errors
    /// Fails only if `assets/catalog/apps.toml` is malformed, which the test suite
    /// checks, so a failure here is a build defect rather than a user-facing one.
    pub fn builtin() -> Result<Self> {
        Self::from_toml(BUILTIN_TOML, "the built-in catalog")
    }

    /// Read a catalog document.
    ///
    /// # Errors
    /// Returns [`Error::Catalog`] for unreadable TOML or a schema version this build
    /// does not understand, [`Error::CatalogRepo`] for an unparseable repository, and
    /// [`Error::BadColor`] or [`Error::BadExtension`] for a value that cannot be used.
    pub fn from_toml(text: &str, source: &str) -> Result<Self> {
        let file: CatalogFile = toml::from_str(text).map_err(|e| Error::Catalog(format!("{source}: {e}")))?;
        Self::from_file(file, source)
    }

    /// Merge an overlay into this catalog; entries with the same `id` are replaced.
    ///
    /// # Errors
    /// Fails if the overlay itself is inconsistent.
    pub fn merge(&mut self, overlay: Self) -> Result<()> {
        for entry in overlay.entries {
            match self.entries.iter_mut().find(|existing| existing.id == entry.id) {
                Some(existing) => *existing = entry,
                None => self.entries.push(entry),
            }
        }
        self.sort();
        self.check_unique("the merged catalog")?;
        // Merging two catalogs is exactly where two entries end up with one colour: each
        // was distinct on its own.
        self.assign_distinct_colors();
        Ok(())
    }

    /// Give every app a tile colour no other app has.
    ///
    /// A catalog entry may name its own colour, and that choice is kept. What cannot be
    /// kept is two entries naming the same one — which is what happens as soon as a
    /// person adds repositories, because a hand-written entry usually names nothing and
    /// the derived default comes from the id. Colliding entries are moved along a
    /// golden-angle hue sequence, which keeps successive colours far apart however many
    /// apps are added.
    ///
    /// This is what makes the grid stay readable at fifty apps, which no fixed palette
    /// of four can do.
    pub fn assign_distinct_colors(&mut self) {
        let mut taken: Vec<Color> = Vec::with_capacity(self.entries.len());
        let mut attempt = 0usize;

        for entry in &mut self.entries {
            if !taken.iter().any(|used| too_close(*used, entry.color)) {
                taken.push(entry.color);
                continue;
            }

            let mut placed = false;
            for _ in 0..POOL_ATTEMPTS {
                let candidate = generated_color(attempt);
                attempt += 1;
                if taken
                    .iter()
                    .all(|used| color_distance(*used, candidate) >= MIN_COLOR_DISTANCE)
                {
                    entry.color = candidate;
                    taken.push(candidate);
                    placed = true;
                    break;
                }
            }
            if !placed {
                // The pool is exhausted, which needs a catalog larger than any person
                // keeps. The entry keeps the colour it had and the tile repeats: a grid
                // of two hundred apps is a worse problem than two similar tiles in it.
                taken.push(entry.color);
            }
        }
    }

    /// The apps, in display order.
    #[must_use]
    pub fn entries(&self) -> &[AppEntry] {
        &self.entries
    }

    /// One app by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&AppEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// How many apps the catalog holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog holds no apps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The ids, in display order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.id.as_str())
    }

    fn from_file(file: CatalogFile, source: &str) -> Result<Self> {
        if file.version > SCHEMA_VERSION {
            return Err(Error::Catalog(format!(
                "{source} is schema version {} but this launcher understands {SCHEMA_VERSION}",
                file.version
            )));
        }

        let mut entries = Vec::with_capacity(file.apps.len());
        for app in file.apps {
            entries.push(resolve(app, source)?);
        }

        let mut catalog = Self { entries };
        catalog.sort();
        catalog.check_unique(source)?;
        catalog.assign_distinct_colors();
        Ok(catalog)
    }

    fn sort(&mut self) {
        self.entries
            .sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
    }

    fn check_unique(&self, source: &str) -> Result<()> {
        let mut seen: BTreeMap<&str, ()> = BTreeMap::new();
        for entry in &self.entries {
            if seen.insert(entry.id.as_str(), ()).is_some() {
                return Err(Error::DuplicateAppId(format!("{} ({source})", entry.id)));
            }
        }
        Ok(())
    }
}

/// Turn one document entry into a validated app.
fn resolve(file: AppFile, source: &str) -> Result<AppEntry> {
    let id = file.id.trim().to_ascii_lowercase();
    let usable_id = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !usable_id {
        return Err(Error::Catalog(format!(
            "{source}: `{}` is not a usable app id",
            file.id
        )));
    }

    let repo = Repo::parse(&file.repo).map_err(|_| Error::CatalogRepo {
        id: id.clone(),
        repo: file.repo.clone(),
    })?;

    let color = match file.color.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(value) => Color::parse(value).ok_or_else(|| Error::BadColor {
            id: id.clone(),
            value: value.to_owned(),
        })?,
        None => fallback_color(&id),
    };

    let kind = match file.kind.as_deref().map(str::trim) {
        None | Some("release") => AppKind::Release,
        Some("link") => AppKind::Link,
        Some(other) => {
            return Err(Error::Catalog(format!(
                "{source}: `{id}` has kind `{other}`, expected `release` or `link`"
            )));
        }
    };

    let install = resolve_install(file.install.unwrap_or_default(), &id, repo.name());
    let launch = resolve_launch(file.launch.unwrap_or_default());
    let detect = resolve_detect(file.detect.unwrap_or_default());

    let name = non_empty(file.name).unwrap_or_else(|| id.clone());
    let tagline = file.tagline.and_then(non_empty).unwrap_or_default();
    let description = file.description.and_then(non_empty).unwrap_or_default();
    let homepage = file
        .homepage
        .and_then(non_empty)
        .unwrap_or_else(|| repo.html_url());

    Ok(AppEntry {
        id,
        name,
        tagline,
        description,
        repo,
        homepage,
        local: file.local.and_then(non_empty),
        color,
        kind,
        install,
        launch,
        detect,
        order: file.order.unwrap_or(UNSORTED_ORDER),
    })
}

fn resolve_install(file: InstallFile, id: &str, repo_name: &str) -> InstallSpec {
    let mut stems: Vec<String> = file
        .stems
        .iter()
        .map(|stem| stem.trim().to_ascii_lowercase())
        .filter(|stem| !stem.is_empty())
        .collect();
    if stems.is_empty() {
        // Sensible default: an asset named after the app, then after the repository.
        stems.push(id.to_owned());
        if repo_name != id {
            stems.push(repo_name.to_owned());
        }
    }
    stems.dedup();

    let mut extensions: Vec<String> = Vec::new();
    for extension in &file.extensions {
        let extension = extension.trim().to_ascii_lowercase();
        // An unknown extension is dropped rather than fatal: a user catalog written
        // for a newer launcher should still load, minus what this build cannot use.
        if extension.is_empty() || !is_known_extension(&extension) {
            continue;
        }
        if !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }

    InstallSpec {
        stems,
        extensions,
        binary: file.binary.and_then(non_empty),
        docs: file.docs.and_then(non_empty),
    }
}

fn resolve_launch(file: LaunchFile) -> LaunchSpec {
    let clean = |values: Vec<String>| -> Vec<String> {
        values
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect()
    };
    LaunchSpec {
        windows: clean(file.windows),
        macos: clean(file.macos),
        linux: clean(file.linux),
    }
}

fn resolve_detect(file: DetectFile) -> DetectSpec {
    DetectSpec {
        paths: file
            .paths
            .into_iter()
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty())
            .collect(),
        command: file.command.and_then(non_empty),
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Split a display name into words on separators and camel-case boundaries.
fn split_words(name: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;

    for character in name.chars() {
        if character == '-' || character == '_' || character.is_whitespace() {
            if !current.is_empty() {
                words.push(core::mem::take(&mut current));
            }
            previous_lower = false;
            continue;
        }
        if character.is_uppercase() && previous_lower && !current.is_empty() {
            words.push(core::mem::take(&mut current));
        }
        previous_lower = character.is_lowercase() || character.is_ascii_digit();
        current.push(character);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// A stable, pleasant colour for an app that does not declare one.
///
/// Adding a repository should not require choosing a colour, but every app still
/// needs to be distinguishable in the grid, so the hue is derived from the id.
fn fallback_color(id: &str) -> Color {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let hue = (hash % 360) as f32;
    hsl_to_rgb(hue, 0.62, 0.52)
}

/// Whether two colours are close enough to read as the same one.
fn too_close(a: Color, b: Color) -> bool {
    color_distance(a, b) < MIN_COLOR_DISTANCE
}

/// How far apart two colours are, as a plain distance in RGB.
fn color_distance(a: Color, b: Color) -> f32 {
    let dr = f32::from(a.r) - f32::from(b.r);
    let dg = f32::from(a.g) - f32::from(b.g);
    let db = f32::from(a.b) - f32::from(b.b);
    (dr * dr + dg * dg + db * db).sqrt()
}

/// The `n`-th colour of the generated sequence.
///
/// Hue advances fastest, then lightness, then saturation, so the closest candidates come
/// first and the sequence only repeats after every combination has been offered.
fn generated_color(n: usize) -> Color {
    let hue = (n % HUE_STEPS) as f32 * (360.0 / HUE_STEPS as f32);
    let band = n / HUE_STEPS;
    let lightness = LIGHTNESS_BANDS[band % LIGHTNESS_BANDS.len()];
    let saturation = SATURATION_BANDS[(band / LIGHTNESS_BANDS.len()) % SATURATION_BANDS.len()];
    hsl_to_rgb(hue, saturation, lightness)
}

/// HSL to sRGB, with saturation and lightness in `0.0..=1.0` and hue in degrees.
fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = (hue.rem_euclid(360.0)) / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let offset = lightness - chroma / 2.0;
    let channel = |value: f32| ((value + offset) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::new(channel(r), channel(g), channel(b))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn the_shipped_catalog_loads_and_holds_the_four_apps() {
        let catalog = Catalog::builtin().unwrap();
        let ids: Vec<&str> = catalog.ids().collect();
        assert_eq!(ids, ["vncli", "vncode", "vn-finance", "vn-mediabus"]);
        assert_eq!(catalog.len(), 4);
    }

    #[test]
    fn every_shipped_app_is_fully_specified() {
        let catalog = Catalog::builtin().unwrap();
        for entry in catalog.entries() {
            assert!(!entry.name.is_empty(), "{} has no name", entry.id);
            assert!(!entry.tagline.is_empty(), "{} has no tagline", entry.id);
            assert!(!entry.description.is_empty(), "{} has no description", entry.id);
            assert!(!entry.install.stems.is_empty(), "{} has no asset stems", entry.id);
            assert!(
                entry.install.stems.contains(&entry.id),
                "{} does not accept an asset named after itself",
                entry.id
            );
            // A launch table is optional — `vncode` is a plugin pack, not a program —
            // but one that exists must cover both non-macOS platforms, or the app
            // would silently be unstartable on half the machines that install it.
            if !entry.launch.is_empty() {
                assert!(
                    !entry.launch.for_os(Os::Windows).is_empty(),
                    "{} declares a launch command but not for Windows",
                    entry.id
                );
                assert!(
                    !entry.launch.for_os(Os::Linux).is_empty(),
                    "{} declares a launch command but not for Linux",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn vncode_is_modelled_as_install_only_because_it_is_not_a_program() {
        let catalog = Catalog::builtin().unwrap();
        let vncode = catalog.get("vncode").unwrap();
        assert!(
            vncode.launch.is_empty(),
            "vncode installs into DeepSeek Harness; it has no launch command"
        );
        assert_eq!(vncode.kind, AppKind::Release);
    }

    #[test]
    fn every_app_that_declares_a_launch_command_names_one_that_can_be_resolved() {
        let catalog = Catalog::builtin().unwrap();
        for entry in catalog.entries().iter().filter(|e| !e.launch.is_empty()) {
            for os in [Os::Windows, Os::MacOs, Os::Linux] {
                let argv = entry.launch.for_os(os);
                if argv.is_empty() {
                    continue;
                }
                // The first element is the program; it must be a placeholder or a
                // plain command name, never something that looks like an absolute
                // path baked in for one machine.
                let program = &argv[0];
                assert!(
                    program.starts_with('{') || !program.contains(['/', '\\']),
                    "{} launches `{program}` on {os}, which is not portable",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn the_four_shipped_apps_are_visually_distinguishable() {
        let catalog = Catalog::builtin().unwrap();
        let mut colors: Vec<String> = catalog
            .entries()
            .iter()
            .map(|entry| entry.color.to_hex())
            .collect();
        colors.sort();
        colors.dedup();
        assert_eq!(colors.len(), catalog.len(), "two apps share a brand colour");
    }

    #[test]
    fn a_minimal_user_entry_gets_sensible_defaults() {
        let catalog = Catalog::from_toml(
            r#"
            [[app]]
            id = "my-tool"
            name = "My Tool"
            repo = "someone/my-tool"
            "#,
            "test",
        )
        .unwrap();
        let entry = catalog.get("my-tool").unwrap();
        assert_eq!(entry.kind, AppKind::Release);
        // Stems default to the id and the repository name.
        assert_eq!(entry.install.stems, ["my-tool"]);
        // Colour is derived, and is a real colour.
        assert!(Color::parse(&entry.color.to_hex()).is_some());
    }

    #[test]
    fn a_user_catalog_overrides_the_builtin_by_id() {
        let mut catalog = Catalog::builtin().unwrap();
        let overlay = Catalog::from_toml(
            r##"
            [[app]]
            id = "vncli"
            name = "vncli (mine)"
            repo = "someone/fork-of-vncli"
            color = "#ff0000"
            "##,
            "overlay",
        )
        .unwrap();
        catalog.merge(overlay).unwrap();

        assert_eq!(catalog.len(), 4, "an override must not add a fifth app");
        let entry = catalog.get("vncli").unwrap();
        assert_eq!(entry.name, "vncli (mine)");
        assert_eq!(entry.repo.slug(), "someone/fork-of-vncli");
        assert_eq!(entry.color, Color::new(0xff, 0x00, 0x00));
    }

    #[test]
    fn a_user_catalog_can_add_an_app() {
        let mut catalog = Catalog::builtin().unwrap();
        let overlay = Catalog::from_toml(
            r#"
            [[app]]
            id = "vecnode-site"
            name = "vecnode.github.io"
            repo = "vecnode/vecnode.github.io"
            kind = "link"
            "#,
            "overlay",
        )
        .unwrap();
        catalog.merge(overlay).unwrap();
        assert_eq!(catalog.len(), 5);
        assert_eq!(catalog.get("vecnode-site").unwrap().kind, AppKind::Link);
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let error = Catalog::from_toml(
            r#"
            [[app]]
            id = "a"
            name = "A"
            repo = "owner/a"
            [[app]]
            id = "a"
            name = "A again"
            repo = "owner/b"
            "#,
            "test",
        )
        .unwrap_err();
        assert!(matches!(error, Error::DuplicateAppId(_)));
    }

    #[test]
    fn a_bad_repository_names_the_entry_that_caused_it() {
        let error = Catalog::from_toml(
            r#"
            [[app]]
            id = "broken"
            name = "Broken"
            repo = "not-a-repository"
            "#,
            "test",
        )
        .unwrap_err();
        match error {
            Error::CatalogRepo { id, repo } => {
                assert_eq!(id, "broken");
                assert_eq!(repo, "not-a-repository");
            }
            other => panic!("expected CatalogRepo, got {other:?}"),
        }
    }

    #[test]
    fn a_bad_colour_is_refused() {
        let error = Catalog::from_toml(
            r#"
            [[app]]
            id = "a"
            name = "A"
            repo = "owner/a"
            color = "rebeccapurple"
            "#,
            "test",
        )
        .unwrap_err();
        assert!(matches!(error, Error::BadColor { .. }));
    }

    #[test]
    fn an_unknown_extension_is_dropped_not_fatal() {
        let catalog = Catalog::from_toml(
            r#"
            [[app]]
            id = "a"
            name = "A"
            repo = "owner/a"
            [app.install]
            extensions = ["zip", "rar", "tar.gz"]
            "#,
            "test",
        )
        .unwrap();
        assert_eq!(catalog.get("a").unwrap().install.extensions, ["zip", "tar.gz"]);
    }

    #[test]
    fn a_newer_schema_is_refused_with_a_readable_message() {
        let error = Catalog::from_toml("version = 99", "test").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("99"), "{message}");
    }

    #[test]
    fn ordering_is_by_order_then_id() {
        let catalog = Catalog::from_toml(
            r#"
            [[app]]
            id = "zebra"
            name = "Zebra"
            repo = "owner/zebra"
            order = 1
            [[app]]
            id = "apple"
            name = "Apple"
            repo = "owner/apple"
            order = 2
            [[app]]
            id = "beta"
            name = "Beta"
            repo = "owner/beta"
            "#,
            "test",
        )
        .unwrap();
        // `beta` declares no order, so it sorts after the curated entries rather than
        // to the front — an app you just pasted should not displace the built-ins.
        assert_eq!(catalog.ids().collect::<Vec<_>>(), ["zebra", "apple", "beta"]);
    }

    #[test]
    fn entries_with_no_order_are_tie_broken_by_id() {
        let catalog = Catalog::from_toml(
            r#"
            [[app]]
            id = "charlie"
            name = "Charlie"
            repo = "owner/charlie"
            [[app]]
            id = "alpha"
            name = "Alpha"
            repo = "owner/alpha"
            "#,
            "test",
        )
        .unwrap();
        assert_eq!(catalog.ids().collect::<Vec<_>>(), ["alpha", "charlie"]);
    }

    #[test]
    fn monograms_split_on_separators_and_camel_case() {
        let entry = |name: &str| AppEntry {
            id: name.to_lowercase(),
            name: name.to_owned(),
            tagline: String::new(),
            description: String::new(),
            repo: Repo::parse("owner/name").unwrap(),
            homepage: String::new(),
            local: None,
            color: Color::new(0, 0, 0),
            kind: AppKind::Release,
            install: InstallSpec {
                stems: vec![],
                extensions: vec![],
                binary: None,
                docs: None,
            },
            launch: LaunchSpec::default(),
            detect: DetectSpec::default(),
            order: 0,
        };
        assert_eq!(entry("vncli").monogram(), "VI");
        assert_eq!(entry("vncode").monogram(), "VE");
        assert_ne!(
            entry("vncli").monogram(),
            entry("vncode").monogram(),
            "two apps in one grid must not share a monogram"
        );
        assert_eq!(entry("vn-finance").monogram(), "VF");
        assert_eq!(entry("vn-mediabus").monogram(), "VM");
        assert_eq!(entry("PhotoCraft").monogram(), "PC");
        assert_eq!(entry("x").monogram(), "X");
    }

    #[test]
    fn a_derived_colour_is_stable_and_not_grey() {
        let first = fallback_color("some-new-app");
        let second = fallback_color("some-new-app");
        assert_eq!(first, second, "the same id must always give the same colour");
        assert!(
            first.r != first.g || first.g != first.b,
            "a derived colour should be a colour, not grey"
        );
    }

    #[test]
    fn colours_that_are_far_apart_are_not_confused() {
        // The other side of the threshold. If this were too eager the pass would move
        // colours that need no moving, and a curated palette would never survive.
        assert!(
            !too_close(Color::new(0x3b, 0x82, 0xf6), Color::new(0x60, 0xa5, 0xfa)),
            "two plainly different blues must not be treated as one colour"
        );
        assert!(
            too_close(Color::new(0x3b, 0x82, 0xf6), Color::new(0x3c, 0x83, 0xf7)),
            "the same colour written two ways must be treated as one"
        );
    }

    #[test]
    fn a_hundred_and_twenty_apps_get_a_colour_each() {
        // None of these declares a colour, so each falls back to one derived from its
        // id — which is exactly where two apps used to end up looking identical.
        let mut text = String::new();
        for index in 0..120 {
            text.push_str(&format!(
                "[[app]]\nid = \"app-{index}\"\nname = \"App {index}\"\nrepo = \"owner/app-{index}\"\n\n"
            ));
        }
        let catalog = Catalog::from_toml(&text, "test").unwrap();
        assert_eq!(catalog.len(), 120);

        let colours: Vec<Color> = catalog.entries().iter().map(|entry| entry.color).collect();
        for (i, a) in colours.iter().enumerate() {
            for (j, b) in colours.iter().enumerate() {
                assert!(
                    i == j || !too_close(*a, *b),
                    "apps {i} and {j} share a tile colour: {a:?} and {b:?}"
                );
            }
        }
    }

    #[test]
    fn a_declared_colour_is_kept_when_nothing_else_claims_it() {
        let catalog = Catalog::from_toml(
            r##"
            [[app]]
            id = "one"
            name = "One"
            repo = "owner/one"
            color = "#16a34a"
            [[app]]
            id = "two"
            name = "Two"
            repo = "owner/two"
            color = "#f97316"
            "##,
            "test",
        )
        .unwrap();
        assert_eq!(catalog.get("one").unwrap().color, Color::new(0x16, 0xa3, 0x4a));
        assert_eq!(catalog.get("two").unwrap().color, Color::new(0xf9, 0x73, 0x16));
    }

    #[test]
    fn a_duplicated_colour_moves_rather_than_repeating() {
        let catalog = Catalog::from_toml(
            r##"
            [[app]]
            id = "one"
            name = "One"
            repo = "owner/one"
            color = "#4c8dff"
            [[app]]
            id = "two"
            name = "Two"
            repo = "owner/two"
            color = "#4c8dff"
            "##,
            "test",
        )
        .unwrap();
        let first = catalog.get("one").unwrap().color;
        let second = catalog.get("two").unwrap().color;
        // The first entry keeps what it declared; the second is moved out of the way.
        assert_eq!(first, Color::new(0x4c, 0x8d, 0xff));
        assert!(!too_close(first, second), "two tiles must not share a colour");
    }

    #[test]
    fn a_local_folder_is_read_and_a_blank_one_is_dropped() {
        let catalog = Catalog::from_toml(
            r#"
            [[app]]
            id = "one"
            name = "One"
            repo = "owner/one"
            local = "C:\\src\\one"
            [[app]]
            id = "two"
            name = "Two"
            repo = "owner/two"
            local = "   "
            "#,
            "test",
        )
        .unwrap();
        let one = catalog.get("one").unwrap();
        assert_eq!(one.local.as_deref(), Some("C:\\src\\one"));
        assert!(one.has_local_folder());

        let two = catalog.get("two").unwrap();
        assert_eq!(two.local, None);
        assert!(!two.has_local_folder());
    }
}
