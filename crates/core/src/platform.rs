//! The machine the launcher is running on.
//!
//! Kept as plain data (rather than `cfg!` scattered through the code) so that the
//! asset-matching rules can be tested against every platform from any platform —
//! which is the only way to be sure the Linux rules are right while developing on
//! Windows.

use core::fmt;

/// An operating system the launcher understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    MacOs,
    /// Linux.
    Linux,
    /// Anything else: the launcher runs, but no release asset can be matched.
    Other,
}

impl Os {
    /// Lower-case name for messages and paths.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOs => "macos",
            Self::Linux => "linux",
            Self::Other => "unknown",
        }
    }

    /// The name a person reads.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Linux => "Linux",
            Self::Other => "this system",
        }
    }

    /// The extension GitHub assets for this system conventionally use.
    #[must_use]
    pub fn default_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Windows => &["zip", "tar.gz", "7z"],
            Self::MacOs => &["zip", "tar.gz", "dmg"],
            Self::Linux => &["tar.gz", "zip", "tar.xz"],
            Self::Other => &["tar.gz", "zip"],
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A CPU architecture the launcher understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    /// 64-bit x86, called `x86_64`, `amd64` or `x64` in asset names.
    X86_64,
    /// 64-bit ARM, called `aarch64` or `arm64` in asset names.
    Aarch64,
    /// 32-bit x86.
    X86,
    /// Anything else.
    Other,
}

impl Arch {
    /// Lower-case name for messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::X86 => "x86",
            Self::Other => "unknown",
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An operating system and architecture pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    /// The operating system.
    pub os: Os,
    /// The CPU architecture.
    pub arch: Arch,
}

impl Target {
    /// The system this build is running on.
    #[must_use]
    pub fn current() -> Self {
        Self {
            os: current_os(),
            arch: current_arch(),
        }
    }

    /// `Windows (x86_64)`, for messages and the About panel.
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} ({})", self.os.label(), self.arch.name())
    }

    /// `windows-x86_64`, for paths and logs.
    #[must_use]
    pub fn slug(&self) -> String {
        format!("{}-{}", self.os.name(), self.arch.name())
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

fn current_os() -> Os {
    if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else if cfg!(target_os = "linux") {
        Os::Linux
    } else {
        Os::Other
    }
}

fn current_arch() -> Arch {
    if cfg!(target_arch = "x86_64") {
        Arch::X86_64
    } else if cfg!(target_arch = "aarch64") {
        Arch::Aarch64
    } else if cfg!(target_arch = "x86") {
        Arch::X86
    } else {
        Arch::Other
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn the_current_target_is_never_other_on_a_supported_build_host() {
        let target = Target::current();
        assert_ne!(target.os, Os::Other, "os should be recognised");
        assert_ne!(target.arch, Arch::Other, "arch should be recognised");
    }

    #[test]
    fn labels_read_well() {
        let target = Target {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        assert_eq!(target.label(), "macOS (aarch64)");
        assert_eq!(target.slug(), "macos-aarch64");
    }
}
