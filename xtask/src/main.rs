//! vn-launcher's task runner.
//!
//! `cargo xtask <task>` — the alias lives in `.cargo/config.toml`. Keeping the
//! checks in one place means CI and a developer's machine run exactly the same
//! thing, which is the only way a CI failure is ever surprising.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let task = arguments.first().map_or("help", String::as_str);

    match task {
        "ci" => ci(),
        "fmt" => cargo(&["fmt", "--all", "--", "--check"]),
        "fmt-fix" => cargo(&["fmt", "--all"]),
        "lint" => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--profile",
            CI_PROFILE,
            "--",
            "-D",
            "warnings",
        ]),
        "test" => cargo(&["test", "--workspace", "--all-features", "--profile", CI_PROFILE]),
        "check" => cargo(&[
            "check",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--profile",
            CI_PROFILE,
        ]),
        "doc" => cargo(&["doc", "--workspace", "--no-deps", "--profile", CI_PROFILE]),
        "version" => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown task `{other}`\n");
            help();
            ExitCode::FAILURE
        }
    }
}

/// The profile the checks run in.
///
/// `dev` optimises every dependency, which is right for the launcher and wrong for a
/// check: the same code takes several times longer to compile. `ci` in `Cargo.toml` is
/// that profile without the optimisations, so `cargo xtask ci` and the workflow still
/// run exactly the same checks on exactly the same code.
const CI_PROFILE: &str = "ci";

/// Everything CI runs, in the order that fails fastest.
fn ci() -> ExitCode {
    for step in [
        &["fmt", "--all", "--", "--check"][..],
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--profile",
            CI_PROFILE,
            "--",
            "-D",
            "warnings",
        ][..],
        &["test", "--workspace", "--all-features", "--profile", CI_PROFILE][..],
    ] {
        let outcome = cargo(step);
        if outcome != ExitCode::SUCCESS {
            return outcome;
        }
    }
    println!("\nall checks passed");
    ExitCode::SUCCESS
}

/// Run a cargo subcommand, inheriting this process's stdio.
fn cargo(arguments: &[&str]) -> ExitCode {
    println!("$ cargo {}", arguments.join(" "));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    match Command::new(cargo).args(arguments).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("`cargo {}` failed ({status})", arguments.join(" "));
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("could not run cargo: {error}");
            ExitCode::FAILURE
        }
    }
}

fn help() {
    println!(
        "\
vn-launcher tasks

  cargo xtask ci        formatting, lints and tests — what CI runs
  cargo xtask fmt       check formatting
  cargo xtask fmt-fix   apply formatting
  cargo xtask lint      clippy over the whole workspace, warnings denied
  cargo xtask test      run every test
  cargo xtask check     type-check without building binaries
  cargo xtask doc       build the documentation
  cargo xtask version   print the workspace version"
    );
}
