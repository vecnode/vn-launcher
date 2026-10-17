# The catalog

The catalog is the list of apps the launcher knows about. It is **data, not code**:
adding a repository is a line in a text file, not a rebuild.

## Where it lives

Three layers are merged, each one over the last, entry by entry, by `id`:

| Layer | Path | Purpose |
|---|---|---|
| Built-in | `assets/catalog/apps.toml` in this repository | Compiled into the binary with `include_str!`. The app that ships. |
| Yours | See below | Whatever you add or override, and whatever you remove. Never overwritten by an update. |
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

## The app that ships

| id | Repository | Colour | Notes |
|---|---|---|---|
| `vncli` | `vecnode/vncli` | `#3B82F6` | The reference case for the release convention, and the only vecnode repository whose releases the launcher would install today. |

One app, on purpose. The launcher used to track four, and three of them published
nothing the launcher could install: `vncode` is a DeepSeek Harness plugin pack, and
`vn-finance` and `vn-mediabus` have no releases at all. A catalog of entries that can only
be pointed at is a catalog that teaches the wrong thing about what the launcher is for, so
they are added from the **Add repository** tab by anyone who wants them instead.

## Removing an app

Removing is the inverse of adding, and it has to work for the apps that ship, which live
in a file compiled into the binary. A merge can add or replace an entry; it cannot delete
one. So removal is written as a fact in *your* file rather than as the absence of an entry
from it:

```toml
version = 1
removed = ["vncli"]
```

`removed` is a list of ids. It is applied **after** every layer has been merged, which is
what makes it beat both the shipped entry and an override of it — the case that made this
a tombstone rather than a deletion. Adding the same repository again takes its id back out
of the list.

Removing also deletes the copy the launcher installed, if it installed one:
`<data>/apps/<id>/`, and nothing else. An app it merely found — on `PATH`, in a folder you
named, in a clone — is never touched. The confirmation names the folder before it goes.
`docs/INSTALL.md` §8 is the full account of what removal may and may not reach.

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
