# Manifest reference

Every field an app entry can set, what it defaults to, and what it changes.

A catalog file is TOML. `docs/CATALOG.md` explains where the file lives and how the
layers merge; this document is the field list.

## The document

```toml
version = 1

[[app]]
id   = "vncli"
name = "vncli"
repo = "vecnode/vncli"
```

| Key | Type | Required | Meaning |
|---|---|---|---|
| `version` | integer | no (default `1`) | Schema version. A file claiming a version this build does not understand is refused with a message naming both. |
| `removed` | array of strings | no | Ids of apps this document removes. Applied after every layer is merged, so a removed app stays removed even when it is one the launcher ships with. See below. |
| `[[app]]` | array of tables | no | The apps. Each is one `[[app]]` block. |

### `removed`

A merge can add or replace an entry but never delete one, and the shipped catalog is
compiled into the binary — so removing an app an app has to be expressible as a fact in
the document rather than as the absence of one:

```toml
version = 1
removed = ["vncli"]
```

- Ids are matched case-insensitively and trimmed; an id that names nothing is ignored
  rather than an error, so a stale tombstone cannot stop the launcher starting.
- The list is applied **after** the user's entries are merged over the built-in ones, and
  after an override of the same id, which is what a person pressing **Remove** expects.
- Adding a repository whose derived id is in the list takes it back out. That is the undo.
- `removed` is a plain value and sits above `[[app]]`, because TOML requires a
  document's values to precede its array of tables. (The serialiser writes them in that
  order.)

## `[[app]]`

| Key | Type | Required | Default | Meaning |
|---|---|---|---|---|
| `id` | string | **yes** | — | Stable machine name. Lower-case letters, digits, `-`, `_`, `.`. Unique within the merged catalog. Used as the directory name under the install root. |
| `name` | string | **yes** | — | Display name. |
| `repo` | string | **yes** | — | `owner/name`, or any of the URL forms `Repo::parse` accepts: a browser URL, a clone URL, an SSH URL, or a deep link such as `/releases`. |
| `tagline` | string | no | empty | One line under the name on the card. |
| `description` | string | no | empty | A paragraph. Reserved for the detail view. |
| `homepage` | string | no | the repository | Where to send someone who wants the project itself. |
| `color` | string | no | derived from `id` | Brand colour, `#rrggbb` or `#rgb`. Drives the app's tile, whose text colour is chosen by contrast. Two entries may not end up with the same one: the launcher moves a colliding colour, so a catalog of any size still has a tile per app. |
| `kind` | string | no | `release` | `release` reads GitHub Releases. `link` opens a page and installs nothing. Anything else is refused. |
| `local` | string | no | absent | A folder on this machine where the app already lives — a clone, a build, or wherever you keep it. The add-repository flow asks for it after the repository. It is stored as written and never required to exist; the launcher does not need the drive mounted to remember the path. |
| `order` | integer | no | after curated entries | Display position; lower sorts first. Ties fall back to `id`. |

### `[app.install]` — how the app is obtained

| Key | Type | Default | Meaning |
|---|---|---|---|
| `stems` | array of strings | `[id, repo-name]` | File-name prefixes this repository's release assets use. See below. |
| `extensions` | array of strings | the running system's default | Archive extensions to accept, **in preference order**. An extension not on the recognised list is dropped rather than refused, so a catalog written for a newer launcher still loads. |
| `binary` | string | the `id` | The executable's name inside the archive, without an extension. `.exe` is appended on Windows when launching. |
| `docs` | string | the repository | Where to send someone when the launcher cannot install it. |

`asset_stems` is accepted as a spelling of `stems`, and `prefer` of `extensions`.

#### Recognised extensions

```
tar.gz  tar.xz  tar.bz2  tar.zst  tgz  zip  7z
dmg  pkg  msi  exe  deb  rpm  appimage  flatpak
```

Note that `dmg`, `pkg`, `msi`, `deb`, `rpm` and `appimage` are *installers*, not
archives. The launcher lists them so a catalog can name them explicitly, but it will
not run one — see `RELEASES.md` §6.

#### How an asset is chosen

For each attached file, the launcher:

1. rejects anything whose extension is not in the accepted list;
2. splits the lower-cased name into **tokens** on every non-alphanumeric character;
3. rejects the file if a token names a *different* operating system or CPU;
4. scores what remains:
   - **+16** if the name begins with one of `stems`;
   - **+8** if it names this operating system;
   - **+4** if it names this architecture, **+2** if it names none;
   - plus its position in `extensions`, best first.

The highest score wins. **A tie is refused**, not broken arbitrarily, and the
launcher says which files tied so a `stems` entry can settle it.

Token matching rather than substring matching is not a detail: `darwin` contains
`win`, so a naive check would offer a macOS build to Windows.

#### Platform tokens understood

| | Recognised spellings |
|---|---|
| Windows | `windows`, `win`, `win32`, `win64`, `msvc`, `mingw` |
| macOS | `macos`, `mac`, `darwin`, `osx`, `apple` |
| Linux | `linux` |
| x86-64 | `x86_64`, `x86-64`, `amd64`, `x64` |
| ARM 64 | `aarch64`, `arm64` |
| x86 | `i686`, `i386`, `x86`, `win32` |

Rust target triples work as they are: `vncli-0.1.0-x86_64-pc-windows-msvc.zip`
contains the Windows and x86-64 tokens.

### `[app.launch]` — how the app is started

```toml
[app.launch]
windows = ["{bin}"]
macos   = ["{bin}"]
linux   = ["{bin}"]
```

Each is an argument vector, first element being the program. Two placeholders are
substituted:

| Placeholder | Becomes |
|---|---|
| `{bin}` | The resolved executable: where detection found the app, or `<install>/<binary>` (plus `.exe` on Windows). |
| `{install}` | The app's install directory. |

The table is **optional, and its absence is meaningful**: an app that installs into
something else rather than running on its own — a plugin pack, a library — declares no
launch command, and the launcher then offers to install it without pretending it can start
it. (`vncode`, which the launcher used to ship, was exactly that shape.)

A vector whose first element contains a path separator is a portability smell; the
catalog test suite checks for it. Use `{bin}` or a bare command name.

### `[app.detect]` — how its presence is noticed

```toml
[app.detect]
command = "vn"
paths   = ["{install}/vn", "{install}/vn.exe"]
```

| Key | Type | Meaning |
|---|---|---|
| `paths` | array of strings | Candidate paths, checked in order. `{install}` is substituted. |
| `command` | string | A command to resolve against `PATH`, honouring `PATHEXT` on Windows. |

The launcher walks `PATH` itself rather than running `which` or `where`, so detection
cannot hang on a subprocess.

**Detection can only ever produce `InstallState::Found`, never `Managed`.** An app
the launcher did not install cannot be updated or removed by it. This is not a
limitation to be worked around; it is the property that stops the launcher deleting
somebody's work. What turns a directory into `Managed` is the launcher's own install
record beside it — see `docs/INSTALL.md` §3.

## Errors

Every problem names the entry that caused it:

| Message | Cause |
|---|---|
| ``catalog entry `x` names an unreadable repository `y` `` | `repo` is not a repository in any accepted form. |
| ``catalog entry `x` has colour `y`, expected `#rrggbb` `` | `color` is not a hex literal. |
| ``two catalog entries share the id `x` `` | The merged catalog has a duplicate `id`. |
| ``catalog entry `x` has kind `y`, expected `release` or `link` `` | `kind` is neither. |
| ``the catalog could not be read: …`` | The TOML is malformed, or the schema version is newer than this build understands. |

None of these stop the launcher starting. They appear as a notice, and the built-in
catalog is used for whatever could not be read.

## A complete example

```toml
version = 1

[[app]]
id          = "vncli"
name        = "vncli"
tagline     = "Personal management CLI and TUI for researchers"
description = """
The `vn` command: git sync, Docker app management, offline AI chat over Ollama,
system and network tooling, and an MCP server.
"""
repo        = "vecnode/vncli"
homepage    = "https://github.com/vecnode/vncli"
color       = "#3B82F6"
kind        = "release"
order       = 1

[app.install]
stems      = ["vncli"]
extensions = ["zip", "tar.gz"]
binary     = "vn"
docs       = "https://github.com/vecnode/vncli#install"

[app.launch]
windows = ["{bin}"]
macos   = ["{bin}"]
linux   = ["{bin}"]

[app.detect]
command = "vn"
paths   = ["{install}/vn", "{install}/vn.exe"]
```

That is the entry `assets/catalog/apps.toml` actually ships for `vncli`, copied here
so the reference and the real thing can be compared.
