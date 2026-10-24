# IBNIZ examples

38 `.ib` files: the five original upstream files plus 33 additional unique
programs from the language manual and two browser galleries. Identical
programs shared between sources are included only once; distinct variants
are kept. Program bodies are unchanged; new files add attribution comments.

**Warning:** some examples flash rapidly and produce loud noise. Start at
low volume, or use `--no-audio`.

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
