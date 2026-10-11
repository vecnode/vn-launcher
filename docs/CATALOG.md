# The catalog

The catalog is the list of apps the launcher knows about. It is **data, not code**:
adding a repository is a line in a text file, not a rebuild.

## Where it lives

Three layers are merged, each one over the last, entry by entry, by `id`:

| Layer | Path | Purpose |
|---|---|---|
| Built-in | `assets/catalog/apps.toml` in this repository | Compiled into the binary with `include_str!`. The four apps that ship. |
| Yours | See below | Whatever you add or override. Never overwritten by an update. |
| *(reserved)* | A manifest in the repository itself | Designed for, not implemented. See "Later" below. |

Where "yours" is, per platform:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\vn-launcher\apps.toml` |
| macOS | `~/Library/Application Support/vn-launcher/apps.toml` |
| Linux | `~/.config/vn-launcher/apps.toml` |

The **Add repository** tab writes this file for you. It is an ordinary TOML document
— the same shape as the one the launcher ships with — so what the launcher adds is
what a person would have typed, and you can edit or delete it by hand without the
launcher minding.

## Merging

An entry with the same `id` as a built-in one **replaces** it entirely. That is what
makes it possible to point `vncli` at your own fork, or give an app a colour you
prefer, without touching the built-in file:

```toml
[[app]]
id    = "vncli"
name  = "vncli (my fork)"
repo  = "someone/vncli"
color = "#ff0000"
```

The merged catalog is re-sorted: by `order`, then by `id`. An entry with no `order`
sorts **after** the curated ones, so adding a repository does not push the built-in
apps down the grid.

If the file cannot be read, the launcher says so in a notice and carries on with the
built-in catalog. A broken catalog is never a failure to start.

## The four apps that ship

| id | Repository | Colour | Notes |
|---|---|---|---|
| `vncli` | `vecnode/vncli` | `#3B82F6` | The only one publishing a release for this platform today. |
| `vncode` | `vecnode/vncode` | `#8B5CF6` | A DeepSeek Harness plugin pack, not a program: install-only, no launch command. |
| `vn-finance` | `vecnode/vn-finance` | `#16A34A` | Publishes no releases yet. |
| `vn-mediabus` | `vecnode/vn-mediabus` | `#F97316` | Publishes no releases yet, and needs a toolchain no launcher should guess at. |

## Adding your own

The shortest useful entry is three lines:

```toml
[[app]]
id    = "my-tool"
name  = "My Tool"
repo  = "someone/my-tool"
```

Everything else is inferred:

- **asset stems** default to the `id` and the repository name;
- **extensions** default to whatever the running system normally ships (`zip` on
  Windows, `tar.gz` elsewhere);
- **colour** is derived from the `id`, so every app is visually distinct without
  anyone choosing;
- **homepage** defaults to the repository;
- **kind** defaults to `release`.

You can override any of it. `docs/MANIFEST.md` lists every field.

## What a repository needs to be installable

Nothing in this file makes an app installable — the repository has to publish
releases the launcher can understand. That agreement is written down separately, in
[`RELEASES.md`](RELEASES.md), and it is the reason downloading is not switched on
yet.

An app whose repository publishes nothing is still worth adding: the launcher tracks
it, reports *"no releases yet"*, and will notice the moment that changes.

## A worked example: a repository whose assets are not named after it

`vecnode/p5js-window` publishes assets called `neo-processing-*`. The stem is not the
repository name, so the entry says so:

```toml
[[app]]
id    = "p5js-window"
name  = "p5js-window"
repo  = "vecnode/p5js-window"

[app.install]
stems      = ["neo-processing"]
extensions = ["zip", "tar.gz"]
```

Without `stems`, no attached file would match and the app would report *"no build for
this system"* — which is the correct answer given what it was told, and the reason
the field exists.

## Later: a manifest in the repository

The schema carries the shape for a `manifest_url` — a repository describing itself,
so that the launcher picks up a new app with no catalog entry at all. It is
deliberately **not implemented**: it requires every repository to adopt the same
convention first, which is what `RELEASES.md` is for. When that convention holds,
the remote manifest is a small addition rather than a redesign, because `Catalog`
already merges layers by `id`.
