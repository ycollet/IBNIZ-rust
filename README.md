# IBNIZ Rust

A standalone Rust implementation of [IBNIZ](https://github.com/viznut/IBNIZ),
Ville-Matias Heikkila's compact audiovisual virtual machine. Use tiny programs
to create visuals and sound. Edit them live or render them offline.
This is an **altered implementation**, not the original C distribution.

![IBNIZ Rust live editor](ibniz-rust.png)

## Features

- Live editor with syntax highlighting, video and audio.
- Fixed-tick playback or optional variable FPS.
- 284 bundled examples with search, credits and source references.
- Single-step debugger, stack/memory inspection and an offline language manual.
- PPM/WAV recording and headless rendering, plus MP4 conversion with FFmpeg.
- NSM session support and a reusable Rust VM library.

## Get started

Download an installer from
[GitHub Releases](https://github.com/ycollet/IBNIZ-rust/releases):

| Platform | Format |
|----------|--------|
| Linux | `.deb` / `.rpm` |
| macOS, Intel or Apple Silicon | `.dmg` |
| Windows | `.msi` |

Builds are unsigned or ad-hoc signed. Verify their source and checksums before
approving installation. See [installation details](DOCUMENTATION.md#installers)
for requirements and security notices.

To run from source, install Rust 1.88+ and the
[platform build dependencies](DOCUMENTATION.md#run), then:

```sh
cargo run --release
cargo run --release -- examples/ibnizdemo.ib
cargo run --release -- -c '^xp M d3r15&*'
```

Click **Examples** to load a program. Use **F1** to run/pause, **F2** to reset,
and **F12** for help. Editing recompiles the program without resetting the VM.

**Warning:** examples can flash rapidly and produce loud noise. Keep volume
low, or use `--no-audio`. FFmpeg is installed separately for MP4 conversion.

For offline rendering without desktop dependencies:

```sh
cargo run --release --no-default-features -- \
  --headless -c '^xp M d3r15&*' --frames 120 \
  --output output --wav output/sound.wav
```

## Documentation

See **[DOCUMENTATION.md](DOCUMENTATION.md)** for the full guide. It covers
installation, controls, playback, debugging, recording, NSM and headless exports.
It also explains the library API, compatibility, development and benchmarks.

Browse the [example catalog](examples/README.md), read the
[original language manual](ibniz.txt), or see [TODO.md](TODO.md) for optimization
measurements and remaining work.

## License

[Zlib](LICENSE). Original IBNIZ by Ville-Matias Heikkila (viznut); the Rust
implementation and frontend are an altered version. Bundled examples retain
their [source credits and license notices](examples/README.md).
