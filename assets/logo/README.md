# The vn-launcher mark

A girdled crystal: a hexagon, a crown ridge running up to the apex, a girdle across
the lower third, and a spine down the centre. Every line is straight, and every
endpoint sits on a hexagon vertex or on that spine — which is what keeps the facets
meeting cleanly instead of crossing.

The geometry lives in the same 24-unit grid `crates/ui/src/icons.rs` draws in, so this
set and the mark inside the application cannot drift apart.

## Which file to use

| Family | For |
|---|---|
| `vn-launcher.svg`, `vn-launcher-<size>.png` | Light backgrounds. Black mark, transparent. |
| `vn-launcher-white.svg`, `vn-launcher-white-<size>.png` | Dark backgrounds. White mark, transparent. |
| `vn-launcher-app-<size>.png`, `vn-launcher-app.rgba` | An application icon: the mark inverted onto the black tile, so it holds up on a light taskbar and a dark one alike. |

## Sizes

16, 24, 32, 48, 64, 128, 256, 512 and 1024 pixels square.

**Prefer the SVG** wherever the target accepts it. It is under 500 bytes, scales
without resampling, and is the only file here that belongs in a print job or a store
listing. The PNGs exist because most places still want one.

Each PNG is drawn at its exact size rather than downscaled from a master. That matters
most at the bottom of the range, where resampling a 1024 down to 16 turns a 2.9-unit
stroke into a grey smear.

## Below about 32 pixels

The faceting costs legibility as the size drops. At 16 pixels the bare mark reads as a
dark hexagonal blob with lighter notches — about as much as any six-line mark can say
at that size. Where a 16-pixel icon has to be recognisable, use
`vn-launcher-app-16.png`: the tile gives the eye a solid silhouette to hold on to,
which the bare mark does not.

## The application icon

`vn-launcher-app.rgba` is 64×64 unmultiplied RGBA, which is exactly the three fields
`egui::IconData` takes. `apps/vn-launcher/src/main.rs` embeds it with `include_bytes!`,
so putting an icon on the window and the taskbar costs the application no image
decoder.

## Regenerating

The set is generated, not hand-drawn, so a change to the mark means regenerating all
thirty files rather than editing one. Keep the geometry here and in `icons.rs` in step
if the mark ever changes: this directory is the distribution copy, not the source of
truth.
