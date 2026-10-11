//! What the launcher remembers between runs.
//!
//! Kept deliberately small. Anything that can be derived — whether an app is
//! installed, what the newest release is — is derived, because a stored copy of a
//! derived fact is a copy that can be wrong.

use serde::{Deserialize, Serialize};
use vnl_ui::ThemeChoice;

/// The few choices worth persisting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// The chosen theme.
    pub theme: ThemeChoice,
    /// How many hours a cached release feed stays fresh.
    pub interval_hours: u32,
    /// Whether pre-releases are offered.
    pub prerelease: bool,
    /// Whether GitHub is contacted at all.
    pub offline: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            // Six hours, four apps: sixteen requests a day against an anonymous
            // allowance of sixty an hour. Comfortable, and the cache means a window
            // reopened ten times in an afternoon costs nothing.
            interval_hours: 6,
            prerelease: false,
            offline: false,
        }
    }
}

impl Settings {
    /// The key the settings are stored under.
    const KEY: &'static str = "vn-launcher.settings";

    /// The lowest and highest intervals the interface offers.
    pub const INTERVAL_CHOICES: [u32; 5] = [1, 6, 12, 24, 168];

    /// Read the settings, falling back to the defaults.
    ///
    /// A settings file written by an older or newer build is treated as "no
    /// settings" rather than as an error: preferences are not worth a failure.
    #[must_use]
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let Some(storage) = storage else {
            return Self::default();
        };
        match eframe::get_value::<Self>(storage, Self::KEY) {
            Some(mut settings) => {
                settings.normalise();
                settings
            }
            None => Self::default(),
        }
    }

    /// Write the settings.
    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::KEY, self);
    }

    /// Bring a loaded value back inside the range the interface can express.
    fn normalise(&mut self) {
        if !Self::INTERVAL_CHOICES.contains(&self.interval_hours) {
            self.interval_hours = Self::default().interval_hours;
        }
    }
}
