//! Starting programs and opening pages.
//!
//! URLs are validated before they are handed to the operating system. That is not
//! ceremony: on Windows the launcher opens a URL through `cmd /C start`, and a URL
//! is a string a catalog author controls, so anything containing shell
//! metacharacters is refused rather than passed along.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// Start a program without waiting for it to finish.
///
/// The child is detached: the launcher returns immediately with its process id, and
/// a reaper thread collects it so a long-running app does not leave a zombie behind.
///
/// # Errors
/// Returns [`Error::Spawn`] when `argv` is empty or the program cannot be started.
pub fn spawn(argv: &[String], working_directory: Option<&Path>) -> Result<u32> {
    let Some(program) = argv.first().filter(|p| !p.trim().is_empty()) else {
        return Err(Error::Spawn {
            program: String::new(),
            reason: "no program was given".to_owned(),
        });
    };

    let mut command = Command::new(program);
    command.args(&argv[1..]);
    if let Some(directory) = working_directory {
        command.current_dir(directory);
    }

    let child = command.spawn().map_err(|error| Error::Spawn {
        program: program.clone(),
        reason: error.to_string(),
    })?;

    let id = child.id();
    reaper(child);
    Ok(id)
}

/// Open a URL in whatever the system uses for that.
///
/// # Errors
/// Returns [`Error::Spawn`] when the URL is not one the launcher will open, or when
/// the system's opener cannot be started.
pub fn open_url(url: &str) -> Result<()> {
    let url = validate_url(url)?;
    let mut command = opener(url);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command.spawn().map_err(|error| Error::Spawn {
        program: opener_name().to_owned(),
        reason: error.to_string(),
    })?;
    reaper(child);
    Ok(())
}

/// Open a directory in the system's file manager.
///
/// Unlike a URL this is a path the launcher owns, so the only check that matters is
/// that it exists — opening nothing at all is the failure worth preventing.
///
/// # Errors
/// Returns [`Error::Spawn`] when the path does not exist or the file manager cannot
/// be started.
pub fn open_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::Spawn {
            program: path.display().to_string(),
            reason: "there is no such directory to open".to_owned(),
        });
    }

    let target = path.to_string_lossy().into_owned();
    let mut command = path_opener(&target);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command.spawn().map_err(|error| Error::Spawn {
        program: opener_name().to_owned(),
        reason: error.to_string(),
    })?;
    reaper(child);
    Ok(())
}

/// Refuse anything that is not a plain http(s) URL.
fn validate_url(url: &str) -> Result<&str> {
    let trimmed = url.trim();
    let scheme_ok = trimmed.starts_with("https://") || trimmed.starts_with("http://");
    let characters_ok = !trimmed.chars().any(|c| {
        c.is_whitespace()
            || c.is_control()
            || matches!(c, '"' | '\'' | '`' | '&' | '|' | '<' | '>' | '^' | '\\')
    });
    if scheme_ok && characters_ok && !has_shell_expansion(trimmed) {
        Ok(trimmed)
    } else {
        Err(Error::Spawn {
            program: trimmed.to_owned(),
            reason: "not a URL the launcher will open".to_owned(),
        })
    }
}

/// Whether a URL contains a `%NAME%`-style expansion.
///
/// Percent-*encoding* (`%20`, `%C3%A9`) is ordinary in a URL and must be allowed;
/// `cmd` also reads `%NAME%` as a variable, and the launcher reaches the system
/// opener through `cmd` on Windows, so a `%` that is not the start of a two-digit
/// escape is refused.
fn has_shell_expansion(text: &str) -> bool {
    let characters: Vec<char> = text.chars().collect();
    let mut index = 0;
    while let Some(&current) = characters.get(index) {
        if current != '%' {
            index += 1;
            continue;
        }
        let escape = characters
            .get(index + 1)
            .zip(characters.get(index + 2))
            .is_some_and(|(a, b)| a.is_ascii_hexdigit() && b.is_ascii_hexdigit());
        if !escape {
            return true;
        }
        index += 3;
    }
    false
}

/// Collect a detached child so it does not linger as a zombie.
fn reaper(mut child: std::process::Child) {
    std::thread::spawn(move || {
        if let Err(error) = child.wait() {
            log::debug!("could not collect a child process: {error}");
        }
    });
}

/// The name of the system's URL opener, for error messages.
fn opener_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "cmd"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    }
}

/// The command that opens a URL on this system.
#[cfg(target_os = "windows")]
fn opener(url: &str) -> Command {
    use std::os::windows::process::CommandExt;

    /// Do not flash a console window when opening a page.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("cmd");
    // The empty first argument is `start`'s title parameter; without it, a quoted
    // URL would be taken as the window title.
    command.args(["/C", "start", "", url]);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// The command that opens a URL on macOS.
#[cfg(target_os = "macos")]
fn opener(url: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(url);
    command
}

/// The command that opens a URL on Linux and the BSDs.
#[cfg(all(unix, not(target_os = "macos")))]
fn opener(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

/// The command that opens a directory on Windows.
#[cfg(target_os = "windows")]
fn path_opener(path: &str) -> Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new("explorer");
    command.arg(path);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// The command that opens a directory on macOS.
#[cfg(target_os = "macos")]
fn path_opener(path: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(path);
    command
}

/// The command that opens a directory on Linux and the BSDs.
#[cfg(all(unix, not(target_os = "macos")))]
fn path_opener(path: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn accepts_ordinary_urls() {
        assert!(validate_url("https://github.com/vecnode/vncli").is_ok());
        assert!(validate_url("http://127.0.0.1:8080/x").is_ok());
        assert!(validate_url("  https://example.com  ").is_ok());
        // Percent-*encoding* is ordinary and must survive.
        assert!(validate_url("https://example.com/a%20b?q=%C3%A9").is_ok());
    }

    #[test]
    fn refuses_a_url_that_could_become_a_shell_command() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://example.com/&calc",
            "https://example.com/ | calc",
            "https://example.com/\"quote",
            "https://example.com/`tick`",
            "https://example.com/^escape",
            "https://example.com/a b",
            "https://example.com/back\\slash",
            // `cmd` would expand this before the URL ever reached a browser.
            "https://example.com/%PATH%",
            "https://example.com/%USERPROFILE%",
            "https://example.com/100%",
            "",
        ] {
            assert!(validate_url(bad).is_err(), "should refuse {bad:?}");
        }
    }

    #[test]
    fn spawning_nothing_is_an_error_not_a_panic() {
        assert!(spawn(&[], None).is_err());
        assert!(spawn(&["   ".to_owned()], None).is_err());
    }

    #[test]
    fn opening_a_bad_url_is_refused_before_any_process_starts() {
        assert!(open_url("javascript:alert(1)").is_err());
    }
}
