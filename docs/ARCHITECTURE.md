# Architecture

Why the code is split the way it is, and where a new piece of behaviour belongs.
`AGENTS.md` states the rules; this file explains the shape they protect.

## The constraint that produces the design

A launcher has one job that is genuinely hard, and it is not drawing a window:

> **Decide which of several attached files belongs on this machine, and be right.**

Everything else follows from taking that seriously. A wrong choice installs a Linux
binary on Windows. A wrong version comparison claims an update that does not exist.
A guessed asset stem turns every new repository into a new special case. So the
decision is separated from the reaching-out, and the reaching-out is separated from
the drawing.

## The shape

```text
        ┌──────────────────────────────────────────────────────┐
        │  apps/vn-launcher                                    │
        │    Shell: owns the World, runs the Worker            │
        │    eframe::App::ui  ──▶  vnl_ui::Ui::show            │
        └───────┬──────────────────────────────┬───────────────┘
                │ Request                      │ Action
                ▼                              ▲
        ┌───────────────────┐          ┌───────┴───────────┐
        │  vnl-native       │          │  vnl-ui           │
        │  Worker thread    │          │  World ──▶ pixels │
        │  HTTP · cache     │          │  Action ◀─ clicks │
        │  detect · launch  │          └───────┬───────────┘
        └───────┬───────────┘                  │
                │ Availability · Detection     │
                ▼                              ▼
        ┌──────────────────────────────────────────────────────┐
        │  vnl-core — pure, IO-free, fully tested              │
        │  Repo · Release · Asset · Tag · Catalog · AppStatus  │
        └──────────────────────────────────────────────────────┘
```

## The three crates

### `vnl-core` — what is true

No filesystem, no network, no clock, no environment. It answers questions as
ordinary functions over ordinary data:

| Question | Function |
|---|---|
| Is this a repository, and what are its URLs? | `Repo::parse` |
| Which release should be offered? | `release::select_latest` |
| Which attached file belongs on this machine? | `asset::choose` |
| Is what is installed older? | `version::is_newer` |
| What should the launcher say about this app? | `status::compute` |
| What does the catalog contain? | `Catalog::builtin`, `Catalog::merge` |
| How long ago was that published? | `time::parse_iso8601`, `time::describe_age` |

The value of the constraint is that every one of those has a test beside it, and
none of them needs a mock, a fixture server or a temporary directory. Asset matching
in particular is tested against the real asset names of all the tracked
repositories, on all three operating systems, from whichever machine happens to be
building — which is the only way the Linux rules are ever verified by someone
developing on Windows.

`time.rs` is the clearest example of the rule paying off: parsing an ISO-8601
timestamp and describing how long ago it was is pure arithmetic over a fixed format,
so it needs no date library, and "now" is a parameter rather than a call to the
clock. That one decision is what makes `describe_age` testable at all.

### `vnl-native` — reaching out

The only crate that touches the world.

| Module | Responsibility |
|---|---|
| `paths` | Where the config, cache and install root are, decided once |
| `http` | HTTPS with the OS trust store, byte caps, honest user agent |
| `cache` | Release feeds on disk, because the launcher is an anonymous guest of a 60-request-per-hour API |
| `feed` | Reads releases and **never fails** — every problem is an `Availability` |
| `detect` | Finds an app already on the machine, and never claims to have installed it |
| `launch` | Starts a process; opens a URL or a directory after validating it |
| `user_catalog` | Reads and writes the person's own catalog file |

Two design choices here are worth stating.

**`Feed` answers with `Availability`, not `Result`.** "The network is down" and
"GitHub is unhappy" are answers like any other, and putting them in the same type as
"here is the release" means no caller has to invent its own idea of what an
unreadable feed means. The interface already has a status for each of them.

**The cache is a convenience, never a source of truth.** A missing, unreadable or
corrupt cache file means "nothing cached", never an error a person must resolve.

### `vnl-ui` — drawing

Knows nothing about the filesystem, the network or processes. It renders a `World`
and returns `Vec<Action>`. That boundary is what lets the entire interface be
exercised against a hand-written `World` in a test, with no server and no fixtures.

- `theme` — every colour, space and corner radius, named by role
- `fonts` — Inter and JetBrains Mono, embedded
- `icons` — drawn with the painter, not shipped as assets
- `widgets` — the vocabulary: cards, badges, buttons, empty states
- `views` — five surfaces, each a function of the `World`
- `app` — the rail, the bar, and the dispatcher

The interface holds only what is a matter of *what is on screen*: which tab, what has
been typed, what is selected. Everything else is derived from the `World` each frame.

## The shell

`apps/vn-launcher` is the only part that knows about all three crates, and the only
part that opens a window. It owns the `World` and runs the worker:

```text
   worker thread ──▶ Response ──▶ Shell.world ──▶ vnl_ui::Ui::show
         ▲                                             │
         └────────────── Request ◀──── Action ◀─────────┘
```

The worker exists because reading four repositories is four HTTP requests, and
seconds of waiting must not happen on the thread drawing the window. It owns the
client, the feed and the cache for the whole session, and reports each app as it
finishes so cards fill in one at a time.

`Worker::drain` reports whether the channel is still open. That is not incidental: a
panic inside a library the worker calls would otherwise leave every card saying
"Checking…" for the rest of the session. A dead worker must be visible.

## Why not a webview

A launcher is a small program that must start instantly and behave identically on
three operating systems. A bundled browser engine is several hundred megabytes and
three more ways to fail, and it would make the interface's behaviour depend on which
WebKit or WebView2 the machine happens to have. The same reasoning rules out
Electron.

Everything the interface needs — rounded surfaces, embedded typefaces, vector
marks, a scrollable list that reflows — egui does directly, and the whole application
including its fonts is a single self-contained binary.

## Two decisions that look small and are not

**Version directories plus a `current` pointer.** Once installation exists, a
release unpacks into `apps/<id>/<version>/` and the pointer flips only after
everything succeeded. An interrupted install therefore leaves the previous version
running, an update is a pointer flip, and a rollback is another one. There is no
"repair" state to reason about because there is no half-installed state.

**`Found` is not `Managed`.** Detection can only ever produce `InstallState::Found`.
An app the launcher did not install cannot be updated or removed by it, because the
launcher has no idea how it was built or where its pieces are. Collapsing the two
would be the single change most likely to delete somebody's work.

## Where a new piece of behaviour goes

| If you are adding… | It belongs in… |
|---|---|
| A rule about releases, versions, assets or statuses | `vnl-core`, with a test |
| A new field a catalog entry can set | `core/src/manifest.rs`, then `catalog.rs`, then `docs/MANIFEST.md` |
| An HTTP request or a filesystem operation | `vnl-native` |
| A new screen or control | `vnl-ui`, driven by `World` and returning `Action` |
| Wiring between the two | `apps/vn-launcher/src/shell.rs` |
| A check that CI should run | `xtask/src/main.rs`, then `.github/workflows/ci.yml` |
