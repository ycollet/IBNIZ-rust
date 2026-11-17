# IBNIZ examples

The catalog contains **284 `.ib` files**:

| Source | Files added |
|--------|-------------|
| Previously bundled examples | 38 |
| Shortest-program and Summerlab collections | 157 |
| TTjm programs and variants | 69 |
| emoc/IBNIZ expressions | 20 |

Identical extracted programs reuse existing files. Distinct variants are
kept. Extracted bodies retain their instructions; only leading collection
comment markers are removed. New files add attribution comments.

**Warning:** some examples flash rapidly and produce loud noise. Start at
low volume, or use `--no-audio`.

## Run an example

Click **Examples** in the desktop app to search all 284 programs or filter by
category. Each entry shows a description, credits, source links and license
notices. Source links are pinned where possible. If a source has no stated
license, the browser says so.

Loading an example resets and pauses playback. The app asks before discarding
edits and suggests `<example>-copy.ib` as a local save path. The browser works
offline, even without this directory.

From the project root:

```sh
cargo run --release -- examples/plasma.ib
cargo run --release -- examples/interactive-julia-sets.ib
cargo run --release --no-default-features -- \
  --headless examples/christmas-demo.ib --frames 120 --output output
```

Move the mouse over the video to explore `interactive-julia-sets.ib`.
The upstream browser galleries use different VM implementations. Rendering,
timing and audio may differ. Complex fractals can exhaust the default
instruction budget. Raise `--max-instructions` if needed.

## Sources and licenses

| Source | Pinned revision | License |
|--------|-----------------|---------|
| [Original examples][original] and [language manual][manual] | `3ba06ae5f2cf5765c0e06874a093f767054b6359` | [Zlib](../LICENSE), Ville-Matias Heikkila |
| [jibniz gallery][jibniz] ([live gallery](https://flupe.github.io/jibniz/)) | `009c312c06938cf30d3b1b50572c51989f0f80ee` | [MIT](LICENSE-jibniz.txt), copyright 2021 flupe; named contributors credited below |
| [IBNIZ-js WebGL gallery][ibnjs] ([live gallery](https://uones.github.io/ibnjs/index.html)) | `03314c9bdb5379527dd0899f168c5e32bd339d41` | [WTFPL v2](LICENSE-ibnjs.txt); named contributors credited below |

These licenses describe the source collections. They do not imply that
maintainers authored every program. Original titles and contributor names
are retained where supplied.

## Original files

| File | Description |
|------|-------------|
| [bitmapzoomer.ib](bitmapzoomer.ib) | Bitmap/data-segment zoomer |
| [ibnizdemo.ib](ibnizdemo.ib) | Viznut's 122-character audiovisual demo |
| [julia.ib](julia.ib) | Julia morpher (real_het, credited in the manual) |
| [mandelbrot.ib](mandelbrot.ib) | Mandelbrot zoomer |
| [oneliners.ib](oneliners.ib) | Commented collection; uncomment one program to run it |

## Added programs

| File | Description / contributor | Source |
|------|---------------------------|--------|
| [42-melody.ib](42-melody.ib) | "42 melody" | Manual |
| [flasher.ib](flasher.ib) | Flasher | Manual |
| [jupiter-storm.ib](jupiter-storm.ib) | Jupiter storm | Manual |
| [jupiter-storm-bw.ib](jupiter-storm-bw.ib) | Jupiter storm, black and white | Manual |
| [mul-texture-zoomer.ib](mul-texture-zoomer.ib) | Two-character multiplication texture zoomer | Manual |
| [tv-noise.ib](tv-noise.ib) | TV noise without sound | Manual |
| [perspective-mapper.ib](perspective-mapper.ib) | Perspective mapper | Manual |
| [sierpinski.ib](sierpinski.ib) | Originally titled "Sierpinski epilepsy" | Manual |
| [polyrhythmic-flasher.ib](polyrhythmic-flasher.ib) | Polyrhythmic flasher slowing down | Manual |
| [sliding-squarewave.ib](sliding-squarewave.ib) | Sliding-down squarewave | Manual |
| [plasma.ib](plasma.ib) | Plasma | Manual |
| [munching-squares.ib](munching-squares.ib) | Munching squares with a Sierpinski harmony | Manual |
| [xor-texture-zoomer.ib](xor-texture-zoomer.ib) | XOR texture zoomer | Manual |
| [music-from-the-video.ib](music-from-the-video.ib) | Music from the original demonstration video | Manual |
| [opening-gate.ib](opening-gate.ib) | Opening gate / FreeFull | Manual |
| [spinny.ib](spinny.ib) | Spinny / FreeFull | Manual |
| [munching-zoomer.ib](munching-zoomer.ib) | Munching squares zoomer | Manual |
| [texture-tunnel.ib](texture-tunnel.ib) | Texture tunnel | Manual |
| [rotozoomer.ib](rotozoomer.ib) | Rotozoomer, AND variant | Manual |
| [interactive-julia-sets.ib](interactive-julia-sets.ib) | Interactive Julia sets | jibniz |
| [xor-texture.ib](xor-texture.ib) | XOR texture | jibniz |
| [merging-thing.ib](merging-thing.ib) | Merging thing??? | jibniz |
| [rose.ib](rose.ib) | Rose / FreeFull | jibniz |
| [mindrape01.ib](mindrape01.ib) | mindrape01 / asiekierka | jibniz |
| [xor-tunnel.ib](xor-tunnel.ib) | XOR tunnel, different rotation from the manual's tunnel | jibniz |
| [animated-mandelbrot-set.ib](animated-mandelbrot-set.ib) | Animated Mandelbrot set | jibniz |
| [julia-sets-morpher.ib](julia-sets-morpher.ib) | Julia sets morpher, alternative to `julia.ib` | jibniz |
| [xorwiggle.ib](xorwiggle.ib) | Xorwiggle / FreeFull | IBNIZ-js |
| [rotozoomer-ibnjs.ib](rotozoomer-ibnjs.ib) | Rotozoomer, XOR variant | IBNIZ-js |
| [bubsy.ib](bubsy.ib) | "Looks funny after a while" / bubsy | IBNIZ-js |
| [four-char-demo.ib](four-char-demo.ib) | Four-character demo / asiekierka | IBNIZ-js |
| [glass.ib](glass.ib) | Glass / asiekierka | IBNIZ-js |
| [christmas-demo.ib](christmas-demo.ib) | Christmas demo / FreeFull | IBNIZ-js |

[original]: https://github.com/viznut/IBNIZ/tree/3ba06ae5f2cf5765c0e06874a093f767054b6359/src/examples
[manual]: https://github.com/viznut/IBNIZ/blob/3ba06ae5f2cf5765c0e06874a093f767054b6359/src/ibniz.txt
[jibniz]: https://github.com/flupe/jibniz/blob/009c312c06938cf30d3b1b50572c51989f0f80ee/index.html
[ibnjs]: https://github.com/uones/ibnjs/blob/03314c9bdb5379527dd0899f168c5e32bd339d41/index.html

## Extracted source collections

### Shortest programs

[shortest_ibniz_programs.txt](shortest_ibniz_programs.txt) adds 40 programs from
this [archived list](https://web.archive.org/web/20190915062423/http://low.fi/~viznut/shortest_ibniz_programs.txt).
Listed alternatives are separate programs. The empty gradient/sawtooth
program is preserved too.

### Summerlab

[summerlab-ibniz-latest.txt](summerlab-ibniz-latest.txt) adds 117 programs from
[Labomedia](https://ressources.labomedia.org/ibniz). These include the complete
multiline sequencers and `trop colle`.

### TTjm

[ttjm_ibniz.md](ttjm_ibniz.md) adds 69 programs and variants from
[TTjm's YouTube channel](https://www.youtube.com/@TTjm-kb6jj).
Search **TTjm** in the GUI to show them.

The source has 41 numbered entries; it does not list 020. Entries 029 and 025
share their explicit `M` audio section across the listed video variants.
Other alternatives remain separate. No missing instructions are inferred.
The ambiguous trailing formulas in 019 and 018 are standalone variants.
The two `etc.` placeholders are excluded. Viznut's original formula credit
in 025 is retained.

### emoc/IBNIZ

[emoc-ibniz.md](emoc-ibniz.md) is an unchanged copy of
[emoc/IBNIZ revision `8004c8abecae10bdd0282154f1a16cb5f6b261e7`](https://github.com/emoc/IBNIZ/blob/8004c8abecae10bdd0282154f1a16cb5f6b261e7/README.md).
It adds 20 expressions, including the 2020 bytefolk rhythms, melodies and
accompaniments.

Each expression remains separate. No `M` separator or other instruction is
inserted. Another 22 occurrences reuse Summerlab programs; two occurrences
repeat earlier entries in the collection.

Search **emoc** to see all 42 distinct runnable expressions. Reused programs
show an additional pinned source link and original title.
`amateurish disintegration` is excluded because lowercase `e` is unsupported.
The source is preserved without rewriting it.

### Extraction rules

[collection-index.tsv](collection-index.tsv) maps every candidate occurrence
to its `.ib` file or explains why it was excluded.

| Status | Meaning |
|--------|---------|
| `included` | New file |
| `reused` | Previously bundled program |
| `duplicate` | Earlier extraction |
| `skipped` | Excluded, with a reason |

Deduplication compares compiled instructions and binary data, not names or
comments. The bitmap zoomer reuses `bitmapzoomer.ib` with its complete data
segment. Line references support ranges and comma-separated spans.
For example, `31,43` selects a video formula and its shared audio section.

Of the 49 excluded Summerlab candidates, 36 use unsupported characters.
The other 13 are incomplete sequencers, empty templates, definitions without
callers, or calls without definitions. Unsupported lowercase instructions are
not silently uppercased or rewritten.

**Compilation does not guarantee a complete video page within the default
instruction budget.**

Each new file records the collection, source lines, source link (pinned where
available), and individual credit where supplied. The TTjm channel link is
not revision-pinned. Its bundled Markdown is kept as the extraction source.

**None of these four collections states an explicit license.** No license is
inferred from this project or unrelated galleries.
