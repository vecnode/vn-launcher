# The vecnode release convention

**Status:** proposed. Nothing in this document is enforced yet.
**Audience:** anyone maintaining a `vecnode` repository that vn-launcher should be able to install.
**Why it exists:** vn-launcher can already read every repository's releases and work out which file belongs on a given machine. It deliberately cannot download, install, update or remove anything yet — because doing that correctly depends on every repository answering the same questions the same way. This document is that agreement.

---

## 1. The problem, stated plainly

A launcher that installs software has to answer six questions about a release before it dares touch a disk:

| # | Question | Today |
|---|---|---|
| 1 | Is this version newer than what is installed? | Answerable — every repo tags `vX.Y.Z` |
| 2 | Which attached file belongs on *this* machine? | Answerable, but only because each repo is special-cased |
| 3 | Is the download intact, and is it the file the maintainer published? | Only `vncode` publishes checksums |
| 4 | Once unpacked, where is the executable? | Varies per repository and is not declared anywhere |
| 5 | How is it started, and with what arguments? | Varies per repository |
| 6 | What does a complete, correct install look like on disk, so it can be undone? | Undefined |

Questions 1, 2 and 5 are answered in the launcher's catalog today (see `docs/CATALOG.md`). Questions 3, 4 and 6 need a decision from the repositories, because a launcher cannot invent an uninstall for a layout it did not choose.

## 2. What the repositories do today

Measured against the live GitHub API, not against intention.

| Repository | Latest release | Assets | macOS | Checksums | Trigger |
|---|---|---|---|---|---|
| `vncli` | `v0.1.0` | `vncli-0.1.0-x86_64-pc-windows-msvc.zip`<br>`vncli-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | **no** | no | `workflow_dispatch` |
| `vncode` | `v0.1.0` | `vncode-0.1.0-{win,mac,linux}.zip`<br>`SHA256SUMS.txt` | yes | **yes** | — |
| `p5js-window` | `v0.1.1` | `neo-processing-{windows.zip,linux.tar.gz}` | no | no | — |
| `droidcli` | `v0.1.0` | *(none attached)* | no | no | — |
| `vn-finance` | — | — | — | — | no tags at all |
| `vn-mediabus` | — | — | — | — | no tags at all |
| `egui-app` | — | — | — | — | no tags at all |

Three things stand out.

**The asset stem is not always the repository name.** `vecnode/p5js-window` publishes `neo-processing-*`. That is not wrong — a repository and a product may have different names — but it means the launcher has to be *told* the stem rather than derive it.

**Suffixes are inconsistent in a way that costs the launcher its confidence.** `-x86_64-pc-windows-msvc` and `-win` are both reasonable; the launcher handles both. But `-windows` next to `-win`, or `-linux` next to `-unknown-linux-gnu`, means every new repository is a new special case and every special case is somewhere a wrong guess can hide.

**Most releases are not reproducible from a tag.** `vncli`'s release workflow is `workflow_dispatch` only: it has to be started by hand against a tag that already exists. A tag can therefore exist with no binaries, or with binaries built from a tree that has since moved on.

None of this is unusual, and none of it is a criticism. It is simply not yet a contract.

## 3. The convention

`MUST` is required for the launcher to install a repository automatically. `SHOULD` is recommended; the launcher works without it but with a worse experience.

### 3.1 Versioning and the release itself

- **MUST** tag releases `v<major>.<minor>.<patch>`, with the leading `v` and no other prefix. `v0.1.0`, not `0.1.0`, not `release-0.1.0`, not `vncli-v0.1.0`.
- **MUST** create a GitHub **Release** for every tag the launcher should see, not merely a tag. The launcher reads releases; a tag alone is invisible to it.
- **MUST** publish the release non-draft. A draft is invisible to the API and therefore to everyone without write access.
- **MUST** mark pre-releases with the pre-release flag *and* a semver pre-release suffix (`v0.2.0-rc.1`). Either alone is fine for a person and confusing for a machine.
- **MUST** trigger publication on the tag, so a release cannot exist without its binaries:

  ```yaml
  on:
    push:
      tags: ['v*']
  ```

  A `workflow_dispatch`-only workflow means a tag can be published and then never built. If manual control is wanted, keep it as an *additional* trigger, not the only one.

- **MUST NOT** re-upload or replace an asset on an existing tag. A checksum is a statement about a specific file; replacing the file silently invalidates every verification anyone has already done. Cut a new patch release instead.

### 3.2 Asset names

Every installable asset **MUST** be named:

```
<stem>-<version>-<os>-<arch>.<ext>
```

| Part | Rules | Examples |
|---|---|---|
| `stem` | Lower-case kebab-case product name. Stable forever — treat it as a public identifier, not a marketing name. | `vncli`, `vncode`, `neo-processing` |
| `version` | The tag without its `v`. | `0.1.0` |
| `os` | One of `windows`, `macos`, `linux`. | `windows`, `macos`, `linux` |
| `arch` | One of `x86_64`, `aarch64`. | `x86_64`, `aarch64` |
| `ext` | `zip`, `tar.gz`, or `tar.xz`. Use `zip` on Windows, `tar.gz` elsewhere. `dmg`/`msi`/`deb`/`rpm`/`AppImage` are **not** accepted — they are installers, and the launcher unpacks rather than runs them. | `zip`, `tar.gz` |

So:

```
vncli-0.1.0-windows-x86_64.zip
vncli-0.1.0-linux-x86_64.tar.gz
vncli-0.1.0-macos-aarch64.tar.gz
```

The launcher accepts several spellings it already sees in the wild — `win` for `windows`, `mac`/`darwin`/`osx` for `macos`, `amd64`/`x64` for `x86_64`, `arm64` for `aarch64`, and `pc-windows-msvc` style Rust target triples — but a repository that uses the canonical spelling above is one the launcher never has to guess about. **Fixed target triples are acceptable** (`x86_64-pc-windows-msvc`) because the launcher reads `windows` and `x86_64` out of them; what it cannot use is an asset that names neither.

**Not every platform is required.** If you do not build for macOS, publish nothing for macOS. The launcher will say *"there is no build for this system"*, which is a useful answer. An asset that is a lie — a Linux binary named `macos` — is not.

### 3.3 What is inside the archive

- **MUST** contain exactly **one** top-level directory, named `<stem>-<version>/`, with everything inside it:

  ```
  vncli-0.1.0/
    vn.exe            (or vn on unix)
    README.md
    LICENSE
  ```

  A single top-level directory is what makes "unpack into a versioned folder" unambiguous. Archives that unpack into the current directory, or into several sibling directories, cannot be installed without guessing.

- **MUST NOT** contain absolute paths, `..` components, or symbolic links that escape the archive. The launcher rejects any entry whose resolved path leaves the extraction root, so this is enforced — but a rejected archive is a failed install, not a security incident avoided quietly.

- **MUST** mark the executables executable in the archive (`tar` preserves the bit; `zip` needs the attribute set explicitly, which GitHub's Windows runners do by default).

- **SHOULD** keep the archive under 200 MB. The launcher caps a download at 1 GB and refuses beyond it.

### 3.4 Checksums

- **MUST** publish a `SHA256SUMS.txt` as a release asset, covering every other asset, in the format `sha256sum` produces:

  ```
  a4a9447b09e37a417c0fb90e7c826c46801c6b32eaf49497b78bbf1842a6b990  vncode-0.1.0-linux.zip
  ```

  `vncode` already does this and is the model to copy.

- **MUST** generate the file after the archives are final, in the same job that uploaded them:

  ```bash
  sha256sum ./*.zip ./*.tar.gz > SHA256SUMS.txt
  gh release upload "$TAG" ./*.zip ./*.tar.gz SHA256SUMS.txt --clobber
  ```

  On Windows runners, `sha256sum` is available through Git Bash; otherwise use `Get-FileHash -Algorithm SHA256`.

The launcher tries three things, in this order, and records which one it used:

1. the release's `SHA256SUMS.txt` — the maintainer's own statement;
2. GitHub's own `digest` field on the asset, which the API reports for files uploaded recently (`sha256:…`);
3. nothing — in which case the file is still downloaded, its hash is **recorded**, and every later update of the same app is checked against that record. The detail view says so rather than implying a verification that did not happen.

**What a checksum does and does not buy you.** It catches a truncated download, a corrupted mirror and a file altered in transit. It does not protect against a compromised maintainer account, because whoever can replace the archive can replace the checksum in the same release. Signing is the answer to that, and it is out of scope here.

### 3.5 Builds worth shipping

These are `SHOULD`s, because they cost the publisher and save everyone else:

- **macOS:** build a universal binary (`--target aarch64-apple-darwin` and `x86_64-apple-darwin`, then `lipo -create`) and publish one `macos-universal` archive. One archive means the launcher never has to pick an architecture on macOS.
- **Linux:** build on the *oldest* glibc you intend to support (`ubuntu-22.04` rather than `ubuntu-latest`). A binary built on a new glibc will not start on an older distribution, and nothing about the archive says so.
- **Windows:** use the MSVC toolchain and link the C runtime statically (`-C target-feature=+crt-static`) so no redistributable is needed.
- **Sign** macOS and Windows builds once a certificate exists. The launcher cannot check signatures yet; the operating system will, on first launch.

## 4. The workflow to copy

One workflow, shared by every repository, parameterised by the stem and the targets. Put it at `.github/workflows/release.yml`.

```yaml
name: Release

# Publishing a tag is what publishes a release. A tag cannot exist without its
# binaries, because the build is what attaches them.
on:
  push:
    tags: ['v*']
  workflow_dispatch:
    inputs:
      tag:
        description: 'Existing tag to build and attach binaries to'
        required: true

permissions:
  contents: write

env:
  # Change this one line per repository. It is the asset stem, and it is public.
  STEM: vncode

jobs:
  build:
    name: ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            os_name: windows
            arch: x86_64
            ext: zip
          - os: ubuntu-22.04          # oldest glibc, not latest
            target: x86_64-unknown-linux-gnu
            os_name: linux
            arch: x86_64
            ext: tar.gz
          - os: macos-latest
            target: aarch64-apple-darwin
            os_name: macos
            arch: aarch64
            ext: tar.gz

    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.inputs.tag || github.ref }}

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --locked --target ${{ matrix.target }}

      - name: Package
        shell: bash
        run: |
          TAG="${{ github.event.inputs.tag || github.ref_name }}"
          VERSION="${TAG#v}"
          NAME="${STEM}-${VERSION}-${{ matrix.os_name }}-${{ matrix.arch }}"
          mkdir -p "dist/${NAME}"
          cp target/${{ matrix.target }}/release/${STEM}* "dist/${NAME}/" 2>/dev/null || true
          cp README.md LICENSE "dist/${NAME}/" 2>/dev/null || true
          # One top-level directory, named <stem>-<version>, and nothing else.
          (cd dist && tar -czf "${NAME}.tar.gz" "${NAME}")
          (cd dist && zip -r "${NAME}.zip" "${NAME}")
          cp "dist/${NAME}.${{ matrix.ext }}" .
          echo "ASSET=${NAME}.${{ matrix.ext }}" >> "$GITHUB_ENV"

      - name: Upload
        shell: bash
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          TAG="${{ github.event.inputs.tag || github.ref_name }}"
          gh release upload "$TAG" "$ASSET" --clobber || \
            gh release create "$TAG" "$ASSET" --title "$TAG" --generate-notes

  checksums:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Collect and publish SHA256SUMS.txt
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          TAG="${{ github.event.inputs.tag || github.ref_name }}"
          gh release download "$TAG" --pattern '*' --dir assets
          rm -f assets/SHA256SUMS.txt
          (cd assets && sha256sum ./* > SHA256SUMS.txt)
          gh release upload "$TAG" assets/SHA256SUMS.txt --clobber
```

Two details in that workflow are load-bearing and easy to lose:

- the **`checksums` job runs after every build finishes**, because a checksum file computed before the last upload is a lie;
- `--clobber` on the *first* upload is what makes a re-run idempotent, but §3.1 still forbids replacing a published asset on a tag people may already have installed.

## 5. How the launcher uses all of this

Mapping the convention onto the code that exists today, so the contract is not abstract:

| Convention | Where it is read |
|---|---|
| `v<major>.<minor>.<patch>` tags | `vnl_core::version::Tag`, `vnl_core::release::select_latest` |
| `<stem>-<version>-<os>-<arch>.<ext>` | `vnl_core::asset::choose`, driven by the catalog's `stems` and `extensions` |
| One top-level directory | the extraction step, which will refuse an archive that has none or several |
| `SHA256SUMS.txt` | `vnl_core::checksum::parse`, already implemented and tested |
| GitHub's `digest` field | `vnl_core::asset::Asset::digest`, already parsed |
| Draft / pre-release flags | `vnl_core::release::Release::is_offerable` |

## 6. What the launcher will do once this holds

Recorded here so the convention can be judged against the mechanism it enables. None of this is implemented yet, and none of it will be until the repositories agree.

### Install

1. Resolve the newest offerable release from the cached feed.
2. Pick the one asset for this machine. If none fits, or two fit equally, **stop and say so**.
3. Download to `<cache>/downloads/<asset>.part`, through a byte cap, with progress and a cancel.
4. Verify against `SHA256SUMS.txt`, then GitHub's `digest`, then record-and-warn.
5. Unpack into `<data>/apps/<id>/<version>.tmp`, rejecting any entry that escapes the root.
6. Normalise: confirm there is exactly one top-level directory and that the declared `binary` exists inside it. If not, stop and say what was found.
7. Rename atomically to `<data>/apps/<id>/<version>/`.
8. Flip the `current` pointer for `<id>`. **This is the last step, and it is atomic**, so an interrupted install leaves the previous version running and nothing half-live.
9. Create the shims: a launcher entry in `<data>/bin/`, plus a Start-menu shortcut, a `.desktop` file or a `.app` bundle as the platform wants.
10. Write the install record: `{ id, version, tag, asset, digest_source, installed_at }`.

### Update

Install the newer version beside the old, flip the pointer, then remove version directories beyond the last two. A rollback is therefore a pointer flip, not a re-download.

### Uninstall

Remove the shims and the menu entries, then the version directories, then the record. **Nothing outside `<data>/apps/<id>/` and the shims the launcher created is ever removed** — an app found on `PATH` that the launcher did not install is never deleted, which is why `InstallState` distinguishes `Found` from `Managed` today.

### Never

- Run an installer with elevated rights, silently or otherwise.
- Run an asset that is a `.msi`, `.dmg`, `.deb`, `.rpm` or `.AppImage`. Those are installers; the launcher unpacks archives.
- Replace a file the launcher did not write.
- Treat a missing checksum as verification.

## 7. Migration checklist

Per repository, in order of value. The first two rows make `vncli` and `vncode` fully installable; the last two are what make `vn-finance` and `vn-mediabus` installable at all.

- [ ] `vncli` — rename assets to `vncli-<version>-<os>-<arch>.<ext>`; add a `macos` build; publish `SHA256SUMS.txt`; trigger on `push: tags`
- [ ] `vncode` — keep `SHA256SUMS.txt`; rename `-win`/`-mac`/`-linux` to `-windows`/`-macos`/`-linux`; add `-<arch>`; trigger on `push: tags`
- [ ] `p5js-window` — rename the stem from `neo-processing` to `p5js-window`, or declare `neo-processing` as the stem in the catalog; add `-<arch>`; publish checksums
- [ ] `droidcli` — attach assets to its existing `v0.1.0` release (or cut `v0.1.1`)
- [ ] `vn-finance` — decide whether it ships a Node bundle or a compiled binary, then tag `v0.1.0`
- [ ] `vn-mediabus` — the MSYS2/GLFW/Lua toolchain is heavy; either publish a prebuilt archive from CI or accept that the launcher will only ever link to it
- [ ] `egui-app` — tag `v0.1.0` if it should be installable

## 8. Open questions

These need a decision from the maintainer before installation is implemented, and are recorded rather than assumed:

1. **A shared reusable workflow.** The five repositories are all Rust or C++ and all want the same six steps. A single `vecnode/.github/.github/workflows/release.yml` with `workflow_call` would remove five copies of the file and the drift between them. Worth doing before the fourth repository copies the template above.
2. **Is `p5js-window` / `neo-processing` the same product under two names?** If the stem is `neo-processing`, it should stay that way everywhere; if it is `p5js-window`, the assets should be renamed. The current state is the one arrangement the launcher cannot infer.
3. **What does `vn-finance` install?** It is a Node 22 application with a `.ts` entry point, not a compiled binary. Either it grows a bundle step (esbuild/`node --experimental-sea-config`), or the launcher gains a runtime-dependency concept — which is a much larger idea than a release convention.
4. **Does `vncode` belong in a launcher at all?** It installs into a DeepSeek Harness profile rather than running on its own, and the launcher models it as install-only for that reason. If it gains its own entry point, that changes.
