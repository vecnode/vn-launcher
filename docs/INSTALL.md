# The install plan

**Status:** a plan, and one part of it shipped. Removing an app works today
(`vnl_native::uninstall`, `vnl_native::user_catalog::remove`). Downloading, verifying,
unpacking, recording and updating are designed below and not implemented.
**Why it exists:** the launcher can already answer *what does GitHub offer* and *which
file belongs on this machine*, and it can already take an app back out again. What it
cannot yet do is act on the first two answers. This document is the plan for the four
operations that would — download, install, update and remove — so they can be judged,
and built, as one mechanism rather than four.
**Read [`RELEASES.md`](RELEASES.md) first if you maintain a repository.** That document is
what a repository must publish; this one is what the launcher will do with it.

---

## 1. What exists, and what this adds

| Piece | Where it is | State |
|---|---|---|
| Reading releases, choosing an asset for this machine | `vnl_core::asset`, `vnl_core::release` | done |
| Deciding the verdict | `vnl_core::status` | done |
| Parsing `SHA256SUMS.txt` | `vnl_core::checksum` | done, unused |
| Finding an app that is already on the machine | `vnl_native::detect` | done |
| Removing an app's catalog entry, with a tombstone | `vnl_native::user_catalog::remove` | **done** |
| Deleting the copy the launcher installed | `vnl_native::uninstall` | **done** |
| Downloading, verifying, unpacking, recording, updating | — | this document |

Removal shipped first, and that is not an accident. It is the only one of the four that
cannot damage anything: until an install exists there is nothing under
`<data>/apps/<id>/` for it to delete, and it refuses to reach anywhere else. It also
forced the two decisions the other three depend on — a tombstone in the catalog, and a
single owned directory per app — to be settled before anything writes into one.

## 2. Why the four arrive together

Installing without updating leaves a person to install by hand twice. Updating without a
record cannot tell what is installed. Removing without a record cannot tell what it may
delete. Each is the other's missing half:

```text
   install ──writes──▶ record ──read by──▶ update ──prunes──▶ old versions
      │                   │                   │
      └──creates──────────┴──────────────────▶ remove ──reads the record, deletes the folder
```

So the unit of work is not "download a file". It is:

> **one record per app, one directory per app, and a pointer that flips atomically.**

Everything below follows from that sentence.

## 3. The layout on disk

```text
<data>/                              %LOCALAPPDATA%\vn-launcher\data   (Windows)
  apps/                              ~/.local/share/vn-launcher        (Linux)
    vncli/                           ~/Library/Application Support/…   (macOS)
      current                        ← a text file: the version in use, e.g. "0.1.0"
      0.1.0/                         one complete, working version
        vn.exe
        README.md
      0.1.1/                         the previous version, kept for rollback
      install.json                   ← the record: what was installed, from where, when
  cache/vn-launcher/cache/
    downloads/
      vncli-0.1.0-x86_64-pc-windows-msvc.zip.part   ← a download in flight
```

Three decisions in that tree are load-bearing.

**A version directory, not a mutable install.** A release unpacks into
`apps/<id>/<version>/` and nothing is ever written into it afterwards. An upgrade is a new
directory; a rollback is a pointer flip; an interrupted install leaves the previous version
whole and still running. There is no "partially installed" state to reason about.

**`current` is a file, not a symlink.** A symlink would be the obvious choice, and it is
the wrong one: creating one on Windows needs either developer mode or elevation, and
`RELEASES.md` forbids the launcher from elevating. A one-line text file holding a directory
name is portable, readable by a person, and — crucially — can be replaced by rename, which
is atomic on every platform. `apps/<id>/` is therefore the only place the launcher ever
mutates in place, and only through a temp file plus a rename.

**`install.json` sits above the version directories**, not inside them, so it is never
deleted by pruning an old version. It is the only thing that can turn a directory the
launcher finds into one it *knows it created*:

```json
{
  "id": "vncli",
  "version": "0.1.0",
  "tag": "v0.1.0",
  "asset": "vncli-0.1.0-x86_64-pc-windows-msvc.zip",
  "asset_url": "https://github.com/vecnode/vncli/releases/download/v0.1.0/…",
  "sha256": "a4a9447b…",
  "digest_source": "sha256sums",
  "installed_at": 1790000000,
  "binary": "vn.exe"
}
```

`digest_source` is one of `sha256sums`, `github_digest`, `recorded` — see §5.3. It is
carried into the interface: an app verified against the maintainer's own checksum file is
not the same claim as one whose hash was merely recorded the first time, and the detail
page says which it is.

**Reading the record is how `InstallState::Managed` finally gets produced.** Today
`detect` can only return `Found`, and that is deliberate: it looks, it does not claim
ownership. The record is what changes it — a directory with a valid `install.json` whose
`id` matches and whose `current` names an existing directory is `Managed`; anything else
is `Found` at best. The distinction is already in the code (`vnl_core::status`), waiting
for a producer.

## 4. Install, step by step

The order is the design. Every step that can fail comes before the step that makes the
install live, and the one irreversible step is last.

| # | Step | On failure |
|---|---|---|
| 1 | Resolve the newest offerable release from the cached feed | Stop. The existing status already says why (`NoReleases`, `CheckFailed`, …) — no new state. |
| 2 | Choose the one asset for this machine | Stop and say so. `AssetChoice::None` means no build for this system; `Ambiguous` names the tie. Never guess. |
| 3 | Check the size and the free space; create `downloads/` | Stop, with the two numbers in the message. |
| 4 | Download to `downloads/<asset>.part` through a byte cap, with progress and a cancel | Keep the `.part` only if a resume is planned; otherwise delete it. The app is untouched either way. |
| 5 | Verify: `SHA256SUMS.txt`, then GitHub's `digest`, then record-and-warn (§5) | Delete the `.part`. Never unpack an unverified file that claimed a checksum. |
| 6 | Unpack into `apps/<id>/<version>.tmp`, rejecting any entry that escapes the root | Delete `.tmp`. The release is refused, and the message says which entry caused it. |
| 7 | Normalise: exactly one top-level directory, named `<stem>-<version>/`, and the declared `binary` must exist inside it | Delete `.tmp`, and say what was found instead — "three top-level entries", "no directory", "no `vn.exe` inside `vncli-0.1.0/`". |
| 8 | Rename `apps/<id>/<version>.tmp` → `apps/<id>/<version>/` | Delete `.tmp` and retry once; a rename within one directory fails only if something else holds it. |
| 9 | Write `apps/<id>/install.json` | Delete the version directory just created. An unpacked directory with no record is an orphan, and the launcher will not adopt it later. |
| 10 | Flip `apps/<id>/current` to the new version, by temp file plus rename | **This is the commit point.** Before it, nothing visible changed; after it, the new version is live. |
| 11 | Offer the shortcut and the PATH-independent entry point (§6) | Not fatal. The app works; the launcher says the shortcut could not be made. |

Two details matter more than they look.

**Step 6 is not a formality.** `zip` and `tar` both permit absolute paths, `..`
components, and symlinks that point outside the archive. The extractor refuses an entry
whose *resolved* path leaves the extraction root, before it writes a byte of it — the check
is on the joined path, computed and compared, not on the string. This is `vnl_core`'s job
(the policy is pure) with `vnl_native` supplying the entries.

**Step 10 can be repeated safely.** If the process dies between 9 and 10, the old version
is still current and the new directory is unreferenced: the next install of the same
version replaces it (`<version>.tmp` then rename over an orphan). There is no recovery
state, because recovery is "install again".

## 5. Verification

Three sources, tried in order, and the record keeps the answer.

### 5.1 `SHA256SUMS.txt`

The maintainer's own statement, and the strongest of the three because it is a claim about
*this* release made by the person who published it. `vnl_core::checksum::parse` already
reads the `sha256sum` format, tolerates `*` binary markers and `./` prefixes, and is
case-insensitive about the hex. A file that names our asset and disagrees with it is a
hard stop: the download is deleted and the install refused.

A file that does *not* name our asset is a warning, not a pass — it means the release was
built without the checksum job, and the launcher falls through to §5.2.

### 5.2 GitHub's `digest`

Newer assets carry a `digest` field (`sha256:…`) on the asset object itself, which the API
reports from the upload. It is not independent of GitHub, but it is independent of the
file the launcher just read, which is exactly what catches a truncated or corrupted
download. `vnl_core::asset::Asset::digest` already parses it.

### 5.3 Record-and-warn

Neither available: the file is still installed, its hash is **recorded** in
`install.json`, and the detail view says *"No checksum was published for this release; the
file's hash was recorded on first download and every later update is checked against it."*
That sentence is a promise the launcher can keep — on the next update of the same app the
recorded hash becomes the reference, and a mismatch stops the update.

**What none of this buys.** A checksum catches a truncated download, a broken mirror and a
file altered in transit. It does not protect against a compromised maintainer account,
because whoever replaces the archive can replace the checksum file in the same release.
Signing is the answer to that; it is out of scope, and the interface must not imply the
launcher has verified a publisher's identity when it has only verified a download.

## 6. Shortcuts, and why they are last

The launcher starts what it installed, so a shortcut is a convenience rather than a
requirement — which is why it is step 11 and not step 1, and why its failure is a notice
rather than a failed install. Each platform wants something different, and only one of the
three is easy:

| Platform | What is created | What it costs |
|---|---|---|
| Linux | `~/.local/share/applications/<id>.desktop` | A text file. Free. |
| Windows | A Start-menu shortcut under `%APPDATA%\Microsoft\Windows\Start Menu\Programs\` | A `.lnk` needs `IShellLink` (COM) or a `.cmd` shim. A `.cmd` shim that launches a GUI app flashes a console window; the COM route adds a dependency. Decide at build time. |
| macOS | `~/Applications/<Name>.app/Contents/{Info.plist,MacOS/…}` | A directory with a plist and a symlink or a copy of the binary. No signing means Gatekeeper will complain, which the launcher cannot fix. |

**Not in this plan:** adding the install directory to `PATH`. That is a system-wide change
to a person's shell, it is not undoable per app, and it is not needed: the launcher knows
where the binary is.

## 7. Update

Install the newer version beside the old one (§4, steps 1–10), then:

1. keep the new `current`;
2. delete version directories beyond the last **two**, oldest first — enough for a
   rollback, not enough to accumulate a gigabyte of history;
3. leave `install.json` alone apart from the fields that changed.

A rollback is therefore a `current` flip to a directory that is still there, with no
network and no re-download. It is not offered in the interface in the first release; it is
the reason the previous version is kept.

**A downgrade is not an update.** `vnl_core::version::is_newer` decides; a release older
than what is installed is `UpToDate`, not an invitation to roll back.

## 8. Remove

What shipped, and what it is allowed to touch:

| Step | Now | Notes |
|---|---|---|
| Write a tombstone in the user's catalog | **done** — `user_catalog::remove` | `removed = ["vncli"]`. Applied after the merge, so it beats both the shipped entry and an override of it. Adding the repository back clears it. |
| Delete `<data>/apps/<id>/` | **done** — `uninstall::remove_installed` | Refuses an id that is not one plain path component, refuses a path whose parent is not the install root, refuses a link, refuses a file. A missing directory is success. |
| Delete the shortcut | later — follows §6 | Only a file the launcher created, identified by the record. |
| Delete the record | later | Falls out of deleting the directory; nothing lives outside it. |

**What removal never does**, and these are the rules the code already holds to:

- deletes anything outside `<data>/apps/<id>/`;
- deletes an app that is `Found` rather than `Managed` — it was not the launcher's to
  delete, which is the whole reason `InstallState` separates the two;
- follows a link, junction or mount point out of the install root;
- touches the download cache of a *different* app.

The tombstone and the deleted directory are independent on purpose. The tombstone can be
undone by adding the repository back; the directory cannot be undone at all. The
confirmation says both, and names the folder before it goes.

## 9. Where the code goes

The layering rules in `AGENTS.md` decide this, and the split is what makes the interesting
parts testable without a network or a disk.

| Piece | Crate | Why there |
|---|---|---|
| Archive entry policy: does this path escape the root? is there exactly one top level directory? is the declared binary inside it? | `vnl-core` (`archive.rs`) | Pure functions over entry names, with tests for `..`, absolute paths, drive letters, backslashes on Linux and forward slashes on Windows. |
| Which of `SHA256SUMS.txt` / `digest` / neither applies, and what the record should say | `vnl-core` (`verify.rs`) | A decision, not an effect. Takes the parsed sums, the digest and the computed hash; returns the source or the mismatch. |
| The record itself: parse, validate, refuse a record whose `id` or `version` disagrees with its directory | `vnl-core` (`record.rs`) | It is data with rules, and getting it wrong is how a launcher deletes the wrong directory. |
| The download: HTTPS, byte cap, progress, cancel, `.part` | `vnl-native` (`download.rs`) | IO. Reuses `http::Client`'s caps and trust store. |
| Unpacking `zip` / `tar.gz` | `vnl-native` (`unpack.rs`) | IO, driven by the core's entry policy. |
| Writing the record, flipping `current`, pruning, deleting | `vnl-native` (`install.rs`, `uninstall.rs`) | Filesystem. |
| Progress, cancel, retry, and the sentences for each failure | `vnl-ui` + `apps/vn-launcher` | States and actions, as everywhere else. |

### 9.1 New dependencies, and the argument against them

Unpacking needs a zip reader and a tar reader. The two candidates are the `zip` crate plus
`tar` + `flate2`, or shelling out to `tar` and `Expand-Archive`.

**Shelling out is refused**, for four reasons that all matter: `PATH` is untrusted (the
same argument that makes `detect` refuse relative entries); the child's output would have
to be parsed to know what happened; there is no way to enumerate an entry *before* it is
written, so the escape check in step 6 could only be done afterwards; and `Expand-Archive`
is PowerShell, which may be absent or restricted.

So the plan is `zip`, `tar` and `flate2` — all pure Rust, all permissively licensed, all
already reachable: `flate2` is in the lock file today as a `ureq` dependency. They are
added to `vnl-native` only, so `vnl-core` keeps its no-dependency property. Every archive
is read through a byte cap as well as an entry cap: a 1 GB download is capped, and so is
the 400 MB it can unpack to.

## 10. The interface, once this exists

The status vocabulary already has the states; installing adds three and the rest are
already drawn:

| State | Card | Detail |
|---|---|---|
| `Available` | **Install**, enabled | The chosen asset, its size, where it would go |
| Installing | **Cancel**, with a progress bar and the byte count | The same, live, plus the step name ("downloading", "verifying", "unpacking") |
| Failed | **Try again**, with the reason | The step it failed at, and what was left behind |
| `UpdateAvailable` | **Update**, enabled | The installed version, the offered one, and the kept previous one |
| `UpToDate` | **Launch** | The record: version, source, verification, when |
| Removing | **Remove** behind a confirmation | Which folder, and what will not be touched |

Three interface rules, unchanged from the rest of the app:

- a disabled button explains itself on hover;
- nothing happens on a schedule — an install starts because a person pressed a button;
- the launcher never claims a verification it did not perform (§5.3).

## 11. Milestones

Each is independently shippable and independently testable. Nothing here is a date.

| # | Milestone | Done when |
|---|---|---|
| M1 | **Remove** | shipped: catalog tombstone, and the launcher's own install folder deleted under the guards in §8 |
| M2 | Archive and record policy in `vnl-core` | entry-escape, single-top-level and record round-trip tests pass on all three platforms, with no IO |
| M3 | Download and verify, with no unpacking | a real release downloads, its hash is checked against `SHA256SUMS.txt`, and a corrupted byte stops it — behind the same cancel and progress the interface will use |
| M4 | Unpack and install, `current` flipping last | installing a real `vncli` release twice leaves one working version, and interrupting at every step leaves the previous one running |
| M5 | Update, prune and rollback | two versions on disk, the older still launchable by flipping `current` |
| M6 | Shortcuts (§6) | one per platform, or a documented decision not to |
| M7 | Verify-and-adopt, or not | decide whether a directory that looks like an install but has no record is ever adopted. The default is **no**: an orphan is the launcher's problem, not the person's |

## 12. Errors a person can act on

Every one of these is a sentence, not a code, and none of them is a panic:

| Situation | What the launcher says |
|---|---|
| The release has no build for this machine | Already answered: *"Release v0.1.0 has no build for this system."* |
| Two files match equally | *"Two attached files match this system equally well (`a.zip`, `b.zip`), so the launcher will not guess. Add a `stems` entry to the catalog to choose."* |
| The checksum file disagrees | *"The file GitHub sent does not match the checksum `vncli` publishes for it. Nothing was installed."* |
| The archive has two top-level directories | *"`vncli-0.1.0.zip` unpacks into two folders (`vncli` and `docs`), so the launcher cannot tell which one is the app. Nothing was installed."* |
| The declared binary is not inside | *"The archive does not contain `vn` inside `vncli-0.1.0/`. It contains `readme.md`."* |
| The disk is full | *"There is not enough room: the download needs 84 MB and 12 MB is free."* |
| Something else is holding the old version | *"`vn.exe` is running, so the update was downloaded and unpacked but not switched on. Close it and try again." |

The last one is worth stating in the plan rather than discovering later: on Windows an
open executable cannot be replaced, and `current` cannot be flipped out from under it. The
answer is to keep the new version on disk and let the next attempt flip the pointer, which
the versioned layout makes possible for free.

## 13. Open questions

Recorded rather than assumed; each one changes the work above if it goes the other way.

1. **Shortcuts on Windows.** COM, or a `.cmd` shim, or nothing? A `.cmd` that starts a GUI
   app flashes a console; a `.lnk` through `IShellLink` is a new dependency and the first
   COM in a crate that has forbidden `unsafe`. Doing nothing is defensible: the launcher
   is the entry point.
2. **Does the launcher ever adopt an existing install?** If a person installs `vncli` by
   hand into `apps/vncli/`, may the launcher take it over? The default is no (M7), but the
   question will be asked the first time somebody does it by accident.
3. **Keeping one previous version, or two?** One is enough for a rollback across a single
   update and halves the disk cost of an app that ships a large archive. Two survives two
   bad releases in a row.
4. **Pre-releases.** The launcher can already offer them. Does an update to a pre-release
   replace a finished release, or install beside it? Replacing is simpler and honest as
   long as the version is in the record and the interface says "pre-release".
5. **A download resume.** `.part` plus HTTP `Range` is a small amount of code and a large
   difference on a bad connection — and a checksum makes a partial file either correct or
   deleted, so the risk is low. Worth doing in M3 if it does not delay it.
