# Eidos brand assets

The mark is a fragmented **E** resolving into a **lozenge**.

The three bars lose their fragmentation as they descend - many mod layers
becoming one view - and the lozenge is *eidos* itself, the Form, hollow with the
resolved core at its centre. It is the program's own semantics, drawn.

## Which file to use

| | |
|---|---|
| `eidos-logo.svg` | the mark, pale ink. **Dark backgrounds only.** |
| `eidos-logo-light.svg` | the mark in the application's own ink (`#2b2018`), for light backgrounds |
| `eidos-icon.svg` | square icon, strata inside the lozenge - **64 px and above** |
| `eidos-icon-small.svg` | square icon, strata dropped - **48 px and below** |

The two icons are not redundant. Below 48 px the interior strata rasterise into
noise and the icon reads as a grey smudge; the simplified one keeps the same
silhouette, so the brand survives the size instead of dissolving at it. The
`png/` exports already apply this rule - `eidos-icon-64.png` and up come from
the full icon, `48` and below from the simplified one.

`png/eidos-app-1024-on-dark.png` is the one to hand to a third party (a store
listing, Nexus Mods) - square, dark background baked in, so it renders as
designed rather than depending on where it lands.

## The application icon

`png/eidos-icon-<size>-on-dark.png` is what gets installed into the icon theme
and embedded in the binary. Dark ground baked in on purpose: the mark is pale
ink and disappears against a light panel, and a taskbar is not somewhere you get
to choose the background.

Three things have to agree or the icon silently does not appear:

| | |
|---|---|
| `APP_ID` in `crates/eidos-gui/src/main.rs` | `eidos` |
| the installed desktop file | `eidos.desktop` |
| the installed icon | `hicolor/<size>/apps/eidos.png` |

A Wayland compositor matches the window's application id against the desktop
file's basename, then reads `Icon=` from it. Miss any leg and a panel shows a
placeholder no matter how many icons are installed - which is exactly what
happened before this was wired: the window announced an *empty* app id, so there
was nothing to match. `hyprctl clients -j` reporting `"class": ""` is that
failure.

`packaging/install.sh` and `packaging/PKGBUILD` install all three. They each
generate the desktop entry inline, because it has to carry the real `Exec` path
for the layout it is installing.

## Colours

| role | dark | light |
|---|---|---|
| ink | `#eaf2f7` | `#2b2018` |
| ground | `#0a0b10` | `#ecdfc2` |

The light pair is the application's real palette (`palette()` in
`crates/eidos-gui/src/main.rs`), so the mark and the program agree.

## Rules

- **Do not distort.** Rescale the `viewBox`; never change width and height
  independently. The lozenge is symmetric on both axes and reads as wrong the
  moment it is not.
- **Do not recolour** beyond the pair above. It is a one-ink mark.
- Keep clear space of one bar-height (16 units) on every side. The `viewBox`
  already carries 14; add the rest with layout, not by cropping.
- Geometry: bar height 16, vertical pitch 40, stem 18 wide, gutter 8, lozenge
  centred at (150,48) with a half-diagonal of 36 on **both** axes, stroke 10.
  Everything sits on a 2-unit grid.

## Regenerating the PNGs

```sh
cd assets/brand
for s in 512 256 128 64; do rsvg-convert -w $s eidos-icon.svg -o png/eidos-icon-$s.png; done
for s in 48 32 16;         do rsvg-convert -w $s eidos-icon-small.svg -o png/eidos-icon-$s.png; done
rsvg-convert -w 1024 eidos-logo.svg -o png/eidos-logo-1024.png
```

Original work, GPL-3.0 with the rest of Eidos.
