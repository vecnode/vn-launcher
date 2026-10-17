<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-white.png">
  <img src="assets/logo-black.png" alt="vn-launcher" width="112">
</picture>

# vn-launcher

**One window for the personal applications.** See what is on your machine.

vn-launcher is a native desktop application optimised for vecnode/vn apps but designed for all apps.

[![License: MIT](https://img.shields.io/badge/license-MIT-3fb950.svg)](LICENSE)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-dea584.svg?logo=rust&logoColor=white)](#requirements)
[![Edition 2024](https://img.shields.io/badge/edition-2024-dea584.svg)](#requirements)
[![egui 0.36](https://img.shields.io/badge/UI-egui%200.36-8b5cf6.svg)](https://github.com/emilk/egui)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-4c8dff.svg)](#supported-platforms)


</div>

---


## Commands

```bash
cargo run --release              # build and run the launcher
cargo build --release            # build it without running it
```

`cargo xtask` runs the same checks CI does:

```bash
cargo xtask ci                   # formatting, lints and tests — exactly what CI runs
cargo xtask fmt-fix              # apply formatting
cargo xtask lint                 # clippy over the workspace, warnings denied
cargo xtask test                 # the test suite
cargo xtask check                # type-check without building binaries
cargo xtask doc                  # build the documentation
cargo xtask version              # print the workspace version
```

The tests that read the real GitHub API are ignored by default:

```bash
cargo test -p vnl-native --test live -- --ignored --nocapture
```

## Repository layout

```text
crates/
  core      vnl-core     pure: catalog, release parsing, asset choice, versions, status
  native    vnl-native   desktop: paths, HTTP, feed cache, detection, process launching
  ui        vnl-ui       egui: theme, fonts, icons, widgets, the five views
apps/
  vn-launcher            the eframe shell that ties them together
xtask/                   cargo xtask ci | fmt | lint | test | version
assets/
  catalog/apps.toml      the catalog that ships
  fonts/                 Inter and JetBrains Mono, under the SIL OFL
docs/                    ARCHITECTURE, CATALOG, INSTALL, MANIFEST, RELEASES
```

## Documentation

| Document | What it answers |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Why the crates are split the way they are |
| [docs/CATALOG.md](docs/CATALOG.md) | Where the catalog lives and how the layers merge |
| [docs/MANIFEST.md](docs/MANIFEST.md) | Every field an app entry can set |
| [docs/INSTALL.md](docs/INSTALL.md) | **The plan for downloading, installing, updating and removing an app** |
| [docs/RELEASES.md](docs/RELEASES.md) | **The release convention every repository must follow** |
| [SECURITY.md](SECURITY.md) | The threat model, the audit findings, and what is still owed |

## Licence

MIT — see [LICENSE](LICENSE).

The bundled typefaces are **Inter** and **JetBrains Mono**, both under the SIL Open
Font License 1.1; their licence texts are in [`assets/fonts/`](assets/fonts).
