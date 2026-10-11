# AGENTS.md — guide for contributors and AI agents

vn-launcher tracks the vecnode applications on a machine, reads each repository's
public GitHub releases, and starts what is already installed. It is written in
**Rust only**: a native egui/eframe application, with **no Tauri, Electron, webview
or JavaScript build step**. Adding one is not a trade-off to be weighed; it is out
of scope.

`docs/ARCHITECTURE.md` explains why the code is shaped the way it is.
`docs/RELEASES.md` is the agreement every repository must keep for the launcher to
install anything. Both are worth reading before a change of any size.

## 1. Workspace map

```text
crates/
  core      vnl-core     pure and IO-free: the catalog, release parsing, asset
                         choice, versions, statuses, timestamp arithmetic
  native    vnl-native   desktop only: paths, HTTP, the feed cache, detection,
                         process launching, the user's catalog file
  ui        vnl-ui       the egui interface: theme, fonts, icons, widgets, views
apps/
  vn-launcher            the eframe shell: owns the world, runs the worker
xtask/                   cargo xtask ci | fmt | lint | test | version
assets/
  catalog/apps.toml      the catalog the launcher ships with
  fonts/                 Inter and JetBrains Mono, under the SIL OFL
docs/                    ARCHITECTURE, CATALOG, MANIFEST, RELEASES
```

**Layering is a rule, not a suggestion.**

- `vnl-core` depends on nothing in the workspace and must never gain a filesystem,
  network or clock dependency. Its value is that every decision it makes is a pure
  function with a test beside it.
- `vnl-native` may use `vnl-core`. It is the only crate that touches the world.
- `vnl-ui` may use `vnl-core`. It must not do IO, read a config file or spawn a
  process — it draws a `World` and returns `Action`s.
- Only `apps/vn-launcher` opens a window, and only it knows about all three.

If a change makes `vnl-core` need to know what time it is, or makes `vnl-ui` want to
read a file, the change is in the wrong place.

## 2. Golden rules

### Never crash (this outranks feature work)

A malformed release payload, a hand-edited catalog, a truncated download or a full
disk must produce an error a person can act on — never a panic.

- Non-test code never panics. No `unwrap()`, `expect()`, `panic!`, `unreachable!`,
  `todo!` or `unimplemented!`. The workspace lints deny all of them.
- Test modules stand those lints down with an inner
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`.
- A panic inside a library the worker calls used to leave the window saying
  "Checking…" for ever. `Worker::drain` now reports a closed channel, and the shell
  clears the spinner and explains. Keep that property: a dead worker must be
  *visible*.

### Everything from the network is hostile

- Read every response through a byte cap (`vnl_native::http::MAX_FEED_BYTES`).
- Treat every field as optional and every array as unbounded until capped.
- Never index; use `get()`, and checked arithmetic for sizes.
- A URL is data a catalog author controls. `vnl_native::launch::open_url` refuses
  anything that is not a plain http(s) URL with no shell metacharacters and no
  `%NAME%` expansion, because on Windows it goes through `cmd`. Do not weaken it.

### Installing other people's software safely

Not implemented yet — see `docs/RELEASES.md` — but the rules that will govern it are
already reflected in the code:

- Verify a download against the release's `SHA256SUMS.txt`, then GitHub's asset
  `digest`, and only then record-and-warn. Never treat a missing checksum as
  verification.
- Never run an installer (`.msi`, `.dmg`, `.deb`, `.rpm`, `.AppImage`). Unpack
  archives; do not execute installers.
- Never elevate, silently or otherwise.
- Never delete anything the launcher did not itself create. This is why
  `InstallState` distinguishes `Found` from `Managed`, and why `detect` never
  produces `Managed`.
- An interrupted install must leave the previous version working. Version
  directories plus an atomically-flipped `current` pointer is the design; keep it.

### The interface says what is true

- A disabled action explains itself on hover. Hiding it would make the app look
  broken; enabling it would be a lie.
- `AppStatus::InstalledUncomparable` exists so the launcher never invents an update
  from a version it cannot compare. Do not "fix" it by falling back to string
  ordering.
- `AppStatus::Available` is deliberately **not** counted by `needs_attention()`: an
  app that was never installed is not an update.

## 3. Commands

```bash
cargo xtask ci        # fmt --check, clippy -D warnings, tests — exactly what CI runs
cargo xtask fmt-fix   # apply formatting
cargo xtask lint      # clippy alone
cargo xtask test      # tests alone
cargo run --release   # the launcher
```

Tests that use the network are ignored by default. Run them deliberately after
touching the HTTP layer or asset matching:

```bash
cargo test -p vnl-native --test live -- --ignored --nocapture
```

## 4. Adding a repository

Do not add an app by editing Rust. Add it to `assets/catalog/apps.toml`, or let the
**Add repository** tab write it into the user's catalog. See `docs/CATALOG.md` and
`docs/MANIFEST.md`.

The catalog is data precisely so that this stays true. If you find yourself wanting
a special case in `catalog.rs` for one repository, the manifest is missing a field.

## 5. Where the tricky parts are

| Concern | File | Why it is subtle |
|---|---|---|
| Choosing the right asset | `core/src/asset.rs` | Token matching, not substring: `darwin` contains `win`. Ties are refused, not guessed. |
| Version comparison | `core/src/version.rs` | Labels that are not versions return `None` rather than an ordering. |
| The verdict | `core/src/status.rs` | One pure function; the interface never re-derives it. |
| The feed | `native/src/feed.rs` | Never fails: every problem is an `Availability`. |
| The cache | `native/src/cache.rs` | The launcher is an anonymous guest of a 60-request-per-hour API. |
| Fonts | `ui/src/fonts.rs` | `set_fonts` lands on the *next* pass, so the shell installs fonts before the first frame. Text drawn with an unbound family panics. |
| Card width | `ui/src/widgets.rs` | A frame's contents must be told the width *minus the padding*. Handing them the full width is what once collapsed the detail page. |
| TLS | `native/src/http.rs` | With default features off, `RootCerts` must be named explicitly or ureq panics on first use. |
| Platform differences | anywhere with `#[cfg]` | CI runs Linux and macOS, and a Windows machine cannot see what they see. See below. |

## 6. What a Windows machine cannot see

CI builds on three platforms, and two of them will disagree with your machine. The
first push of this repository failed on Linux *and* macOS with `unused_mut`, in code
that compiled perfectly on Windows:

```rust
let mut out = vec![directory.join(command)];

#[cfg(windows)]
if Path::new(command).extension().is_none() {
    out.push(/* … */);       // on Linux this whole block does not exist
}

out                          // …so `mut` is unused, and CI denies warnings
```

An attribute on a **statement** deletes it on the other platforms, which can leave a
binding unused, a variable unread or a branch unreachable. An attribute on an **item**
— a function, a `use`, a `struct` — cannot. When behaviour genuinely differs per
platform, write two functions of the same name under `#[cfg(windows)]` and
`#[cfg(not(windows))]`, each self-contained; when only a value differs, use `cfg!`.

Before pushing anything with a `#[cfg]` in it, hold the file to a target you are not
on:

```bash
rustup target add x86_64-unknown-linux-gnu
cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

That needs a Linux C cross-compiler for `ring`, which comes in through `ureq`, so it
may stop in a dependency rather than in your code. It is still worth running: if it
reaches your crates, it has checked them. When it cannot, a standalone file carrying
the same `#[cfg]` shape compiled with `rustc --target x86_64-unknown-linux-gnu
--crate-type lib -D warnings` is enough to reproduce or clear the question.

## 7. Style

- British spelling in prose and identifiers (`colour`, `behaviour`, `catalogue` in
  prose; `color` only where it is a field name a person might type).
- Comments explain *why*, never *what*. If a line needs a comment to say what it
  does, rewrite the line.
- A doc comment on every public item; `missing_docs` is a warning and CI denies
  warnings.
- Prefer a named constant over a literal, and a named type over a tuple.
