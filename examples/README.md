# IBNIZ examples

264 `.ib` files: the 38 previously bundled examples plus 157 complete,
compiler-valid programs extracted from the shortest-program and Summerlab
collections, and 69 TTjm programs/variants. Identical extracted programs reuse existing files; distinct
variants are kept. Program bodies are unchanged except for removing the
collection's leading comment markers; new files add attribution comments.

**Warning:** some examples flash rapidly and produce loud noise. Start at
low volume, or use `--no-audio`.

In the desktop app, click **Examples** to search and filter all 264 embedded
programs by category. The browser includes descriptions, contributor credits,
source links (pinned where available) and collection license notices (including an explicit
unspecified-license notice where the bundled source provides no license). Loading an example resets
and pauses playback, asks before discarding edits, and suggests a local
`<example>-copy.ib` save path. The browser works offline without this directory.

From the project root:

```sh
cargo run --release -- examples/plasma.ib
cargo run --release -- examples/interactive-julia-sets.ib
cargo run --release --no-default-features -- \
  --headless examples/christmas-demo.ib --frames 120 --output output
```

Move the mouse over the video to explore `interactive-julia-sets.ib`.
The browser galleries use somewhat different VM implementations; rendering,
timing and audio can differ. Complex fractals may exhaust the default
instruction budget; raise `--max-instructions` if needed.

## Sources and licenses

| Source | Pinned revision | License |
|--------|-----------------|---------|
| [Original examples][original] and [language manual][manual] | `3ba06ae5f2cf5765c0e06874a093f767054b6359` | [Zlib](../LICENSE), Ville-Matias Heikkila |
| [jibniz gallery][jibniz] ([live gallery](https://flupe.github.io/jibniz/)) | `009c312c06938cf30d3b1b50572c51989f0f80ee` | [MIT](LICENSE-jibniz.txt), copyright 2021 flupe; named contributors credited below |
| [IBNIZ-js WebGL gallery][ibnjs] ([live gallery](https://uones.github.io/ibnjs/index.html)) | `03314c9bdb5379527dd0899f168c5e32bd339d41` | [WTFPL v2](LICENSE-ibnjs.txt); named contributors credited below |

These are the source collections' licenses, not a claim that the collection
maintainers authored every program. Original titles and contributor names
are retained where provided by the sources.

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

- [shortest_ibniz_programs.txt](shortest_ibniz_programs.txt), from this
  [archived list](https://web.archive.org/web/20190915062423/http://low.fi/~viznut/shortest_ibniz_programs.txt):
  40 new programs. Listed alternatives are separate programs; the empty
  gradient/sawtooth program is preserved too.
- [summerlab-ibniz-latest.txt](summerlab-ibniz-latest.txt), from
  [Labomedia](https://ressources.labomedia.org/ibniz): 117 new programs,
  including the complete multiline sequencers and `trop colle`.
- [ttjm_ibniz.md](ttjm_ibniz.md), from
  [TTjm's YouTube channel](https://www.youtube.com/@TTjm-kb6jj): 69 programs
  and variants from 41 numbered entries (the source does not list 020).
  Search **TTjm** in the GUI to show this collection. Entries 029 and 025
  share their explicit `M` audio section across the listed video variants;
  other alternatives remain separate. No missing `M` or other instructions
  are inferred: the ambiguous trailing formulas in 019 and 018 are standalone
  variants. The two `etc.` placeholders are excluded. Viznut's original
  formula credit in 025 is retained.

[collection-index.tsv](collection-index.tsv) maps every candidate occurrence
to its `.ib` file or records why it was excluded. `included` means a new
file, `reused` means a previously bundled program, and `duplicate` points
to an earlier extraction. Deduplication compares compiled instructions and
binary data, not names or comments. The bitmap zoomer reuses the original
`bitmapzoomer.ib`, retaining its complete binary data segment.
Line references can combine ranges and comma-separated spans, for example
`31,43` selects a video formula and its shared audio section.

49 Summerlab candidates are excluded: 36 use unsupported characters and
13 are incomplete sequencers, empty templates, definitions without callers,
or calls without their definitions. Unsupported lowercase instructions are
not silently uppercased or rewritten. Compiling successfully does not imply
that a program can complete video within the default instruction budget.

Each new file records the collection, source lines, source link (pinned where
available), and individual credit where supplied. The TTjm channel link is
not revision-pinned; the bundled Markdown is preserved as the extraction
source. None of these three collections declares an
explicit license; no license is inferred from this project's license or
from unrelated galleries.
