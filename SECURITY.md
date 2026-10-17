# Security

What vn-launcher does, what it deliberately does not do, what an audit of revision
`0.1.0` found, and what is still owed.

## Reporting

Use GitHub's **private vulnerability reporting** on this repository (the *Security*
tab → *Report a vulnerability*). Please do not open a public issue for anything
exploitable.

Include what you did, what happened, and what you expected. A proof of concept is
welcome but not required — a clear description of a reachable path is more useful than
a payload that only works on your machine.

## Threat model

A launcher is a program that decides what to run and, eventually, what to download.
That makes two things worth protecting:

1. **The machine**, from a repository, a release payload, or a catalog entry that
   turns out to be hostile.
2. **The person**, from being told something untrue — that a file is verified when it
   is not, that an update exists when it does not, that an app is one thing when it is
   another.

What is explicitly **not** in the threat model:

- An attacker who can already write to the user's own configuration, cache or data
  directories. At that point they can replace the launcher's binary too.
- An attacker who controls the local network *and* has installed a root certificate
  the operating system trusts. The launcher defers to the OS trust store on purpose
  (see *Accepted risks*).
- A malicious catalog file the person deliberately wrote. The catalog is executable
  configuration by design; it is theirs.

## Properties the design gives you

These are structural, not incidental, and each is worth keeping:

**There are no credentials.** No token field, no login, no OAuth, no session, no
stored secret. Every request goes to GitHub's public API as an anonymous guest. This
is the single largest reduction in risk available to a launcher, and it also removes
the entire class of "leaked token" bugs. It is why the feed is cached aggressively —
the trade for having no account is a 60-request-per-hour allowance.

**No webview, no JavaScript.** The interface is egui, drawn directly. There is no HTML
parser, no script engine and no browser engine in the process. Release notes from a
repository are rendered as plain text and are never interpreted as markup, so a
release body cannot become an injection vector.

**No `unsafe`.** `unsafe_code = "forbid"` workspace-wide.

**No panics on production paths.** `unwrap`, `expect`, `panic!`, `unreachable!`,
`todo!` and `unimplemented!` are denied by the workspace lints and enforced in CI.
Every one of them in the tree is inside a `#[cfg(test)]` module or an integration test.

**Every response is bounded.** Reads go through `MAX_FEED_BYTES` (4 MB) and
`MAX_PROBE_BYTES` (1 MB), and the parser caps releases, assets and every individual
string within them.

**URLs are validated before the operating system sees them.** On Windows a URL is
opened through `cmd /C start`, so `vnl_native::launch::open_url` refuses anything that
is not a plain `http(s)` URL, rejects shell metacharacters, and rejects `%NAME%`
expansion while still allowing percent-*encoding*. This matters because the URL can
come from a catalog file.

**Nothing is written outside the launcher's own directories.** The catalog and the
cache are the only files created or replaced, both under the platform's own config and
cache locations, and the catalog write goes through a temporary file and a rename.

**Removal can only reach one directory.** `vnl_native::uninstall` takes an **id**, never
a path: it refuses an id that is not exactly one plain path component (checked against
both separators on every platform), refuses a target whose parent is not the install
root, refuses a link, and refuses a file. Anything it did not create is therefore out of
reach — an app found on `PATH`, in a folder the person named, or in a clone is never
deleted, and an app that fails those checks is reported rather than removed. The catalog
tombstone is written *before* the directory, so a failed catalog write deletes nothing.

## Audit of revision 0.1.0

Performed on the initial commit. Every finding below was fixed in the same revision
unless marked otherwise. Severity is assessed on reachability and impact together, not
on the shape of the bug.

### VNL-1 — Reachable panic on non-ASCII catalog colour · **High** · fixed

`Color::parse` validated by **byte** length and then sliced by **byte** index. A value
such as `"€€"` is six bytes long and two characters wide, so it took the six-digit
branch and then sliced at byte 2 — which falls *inside* the first `€`. That is a
`str` index panic, not an error return.

The value comes from `color` in a catalog file, which a person edits by hand and which
the add-repository flow appends to. A single line was therefore enough to crash the
launcher every time it started, with no way out from inside the application.

Fixed by rejecting non-ASCII before any slicing, which makes every later index a
character boundary by construction, with a regression test over multi-byte, 3-byte,
4-byte and zero-width inputs.

### VNL-2 — A hostname could be read as an owner name · **Medium** · fixed

`Repo::parse` allowed dots in the owner segment. `https://github.com.evil.com/owner`
therefore parsed as a repository owned by `github.com.evil.com`, and the
add-repository preview displayed exactly that — a string that reads like GitHub and is
not. The request would have gone to the real `api.github.com` and 404'd, so nothing
dangerous happened, but a launcher should not show a person a repository name that is
really a hostname.

Fixed by enforcing GitHub's own rule: an owner may contain only letters, digits and
hyphens, and may not begin or end with a hyphen. Repository names keep their wider
charset, so `vecnode/vecnode.github.io` remains valid.

### VNL-3 — Relative `PATH` entries were searched · **Medium** · fixed

`detect::find_on_path` walked every `PATH` entry. A relative entry — and an empty one,
which every platform reads as the current directory — would make the launcher report
an app as present because a file of that name sat in whatever directory the launcher
was started from, and then offer to launch it. The classic untrusted-search-path
mistake.

Fixed by searching absolute entries only.

### VNL-4 — Unbounded strings inside the bounded body · **Medium** · fixed

The 4 MB response cap bounds the *document* but not any single string inside it: one
JSON field could be the whole 4 MB. Tags, release titles, asset names and URLs were all
taken verbatim, and a tag of that size would be parsed, compared, written to the cache
and laid out on screen — a way to make the interface stop responding.

Fixed with per-field caps. A tag beyond `MAX_TAG_CHARS` drops its release rather than
being truncated, because a truncated version would be an untrue version. Titles are
truncated (display only); over-long asset names and URLs are dropped; assets per
release are capped.

### VNL-5 — `panic = "abort"` removed a safety net · **Low** · fixed

The release profile aborted on panic. That is a defensible default, but not here: the
feed runs on its own thread, and `Worker::drain` detects that the thread is gone and
tells the person instead of leaving the window saying "Checking…" for ever. Aborting
throws that away — a panic inside a dependency takes the whole process instead.
Removed.

### Verified clean

- **Dependency advisories.** `cargo audit` against the lockfile: **0 vulnerabilities,
  0 warnings**, 471 dependencies, database of 1 296 advisories.
- **`unsafe`.** None.
- **Panics outside tests.** None.
- **Command injection.** The only shell is `cmd /C start` on Windows, behind the URL
  validation described above. Process spawning elsewhere uses `std::process::Command`
  with an argument vector, never a shell string.
- **Path traversal.** No user-controlled value reaches a path join. Catalog ids are
  restricted to ASCII alphanumerics, `-`, `_` and `.`, and the one deletion that joins an
  id onto a path re-checks that it is a single plain component on both separator
  conventions; URLs are parsed structurally; `{install}` is always the launcher's own
  directory.
- **Credential handling.** Nothing to handle.
- **Secret material in the tree.** None.

## Accepted risks

Each of these is a decision, not an oversight.

**The catalog is trusted input.** A catalog entry's `launch` value is an argument
vector the launcher will execute on request. That is what the file is *for*. The
mitigating property is that the add-repository flow never writes a `launch` command —
`user_catalog::append` writes an id, a name and a repository, and the inferred launch
spec is empty — so adding a remote repository cannot introduce one. A launch command
requires a person to write it into a file they own.

**The feed cache is trusted input.** A tampered `feeds.json` could make the launcher
display a version that does not exist. It cannot cause execution, and with downloading
not yet implemented it cannot cause a download either. When installation arrives, the
cache must remain advisory and must never be the source of a download URL.

**TLS defers to the operating system.** Certificates are verified with
`rustls-platform-verifier` against the OS trust store rather than a bundled root list.
That is the licensing-safe choice and the one that keeps a corporate proxy working, and
it means a root certificate the user has installed is trusted. The alternative —
bundling Mozilla's roots — trades one trust decision for another.

**Files are created with default permissions.** Nothing the launcher stores is secret,
so no explicit mode is set. If a token is ever added, this becomes a finding: it would
need the platform keychain, or `0600`.

**PATH entries can be attacker-controlled.** Detection searches absolute `PATH`
entries, so a directory early in `PATH` can shadow a real tool. That is how `PATH`
works; the launcher reports *where* it found something so the person can see it, and
only launches on an explicit click.

## What is not implemented, and what that owes

Downloading, verifying, extracting, installing and updating are all absent. The plan for
them is `docs/INSTALL.md`; the requirements below are recorded in `docs/RELEASES.md` §6
and must be met — and audited — before any of them ship.

**Removing has shipped**, and owes less than the others because it has nothing to unpack
or verify: it writes a tombstone in the user's catalog, then deletes
`<data>/apps/<id>/` if it exists, under the guards above. It cannot delete an app the
launcher did not install, because until installing exists it has installed nothing, and
because the checks above do not depend on that being true.

- **Extraction must reject** any archive entry whose resolved path escapes the
  extraction root: `..` components, absolute paths, and symlinks pointing outside.
- **Verify before execute.** `SHA256SUMS.txt`, then GitHub's asset `digest`, then
  record-and-warn. A missing checksum is not verification, and the interface must say
  which of the three happened.
- **Never run an installer.** `.msi`, `.dmg`, `.deb`, `.rpm` and `.AppImage` are
  installers, not archives. Unpack; do not execute.
- **Never elevate**, silently or otherwise.
- **Install atomically.** Version directories plus a `current` pointer flipped last, so
  an interrupted install leaves the previous version working and a rollback is a
  pointer flip.
- **Never delete** anything the launcher did not create. This is why `InstallState`
  separates `Found` from `Managed` and why detection never produces `Managed`.
- **Never let the cache choose a download URL** without the release feed agreeing.

## Reproducing this audit

```bash
cargo audit                                    # dependency advisories
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace                         # includes the regression tests above
cargo test -p vnl-native --test live -- --ignored --nocapture   # real API, needs network
```

The panic in VNL-1 has a targeted regression test
(`color::tests::multi_byte_input_is_rejected_rather_than_panicking`), as do VNL-2
(`repo::tests::a_hostname_is_never_read_as_an_owner`) and VNL-4
(`release::tests::an_absurd_tag_drops_its_release_rather_than_freezing_the_interface`).
