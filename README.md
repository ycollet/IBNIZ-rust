# IBNIZ Rust

A standalone Rust implementation of [IBNIZ](https://github.com/viznut/IBNIZ),
Ville-Matias Heikkila's compact audiovisual virtual machine. Includes a live
code editor, realtime video and sound, a reusable VM library, and headless
PPM/WAV rendering. This is an **altered implementation**, not the original C
distribution.

## Run

Install Rust 1.88 or newer, then:

```sh
cargo run --release
cargo run --release -- examples/ibnizdemo.ib
cargo run --release -- -c '^xp M d3r15&*'
```

The desktop uses egui/OpenGL and CPAL rather than SDL. On macOS, install Xcode
Command Line Tools (`xcode-select --install`). On Debian/Ubuntu, the desktop
build needs `build-essential pkg-config libasound2-dev libx11-dev libxi-dev
libxcursor-dev libxrandr-dev libxinerama-dev libgl1-mesa-dev libwayland-dev
libxkbcommon-dev`. Windows needs the Rust MSVC toolchain and Visual Studio C++
build tools.

Sound starts at 20% volume: IBNIZ programs can produce loud, abrupt noise.
`--no-audio` disables the output device. Audio initialization failures are
shown in the application; editing and video remain available.

### Linux desktop integration

`assets/ibniz.svg` is the scalable application icon. The desktop launcher
`assets/ibniz.desktop` uses the icon name `ibniz`, matching the application's
Wayland app ID and X11 window class.

To install for the current user:

```sh
cargo build --release
install -Dm755 target/release/ibniz "$HOME/.local/bin/ibniz"
install -Dm644 assets/ibniz.svg "$HOME/.local/share/icons/hicolor/scalable/apps/ibniz.svg"
install -Dm644 assets/ibniz.desktop "$HOME/.local/share/applications/ibniz.desktop"
```

Ensure `$HOME/.local/bin` is on your desktop session's `PATH`.
The native window embeds `assets/ibniz.png`, so it does not need external
asset files at runtime. To regenerate this PNG after changing the SVG,
install librsvg and run:

```sh
rsvg-convert --width 256 --height 256 --output assets/ibniz.png assets/ibniz.svg
```

## Live editor

| Control | Action |
|---------|--------|
| F1 | Run / pause |
| F2 | Reset memory, stacks and time |
| F12 | Show / hide help (shortcuts and instruction reference) |
| Tab | Show / hide editor |
| Ctrl/Cmd+S | Save to the file path above the editor |
| Ctrl+Up / Ctrl+Down | Increment / decrement the hex digit under the cursor, with carry |
| Esc | Close help, or exit when help is hidden |
| Load / Save | Open / save the specified UTF-8 `.ib` file |

The editor supports platform-standard selection, clipboard, word navigation
and undo/redo. Changes recompile immediately **without resetting VM state**.
Compile errors suspend playback until corrected; resetting uses the last
successfully compiled program. A `T` instruction stops execution; use F2 and
F1 to start it again.

The status bar shows stack pointers, video mode, time, instruction count and
audio health. Slow programs are bounded by `--max-instructions` per tick
(default 4,000,000); budget exhaustion is reported instead of freezing the UI.
Playback slows down if your machine cannot produce a tick in 1/60 second.

Mouse coordinates, buttons and modifiers feed `U`. Hide the editor to send
typed characters to the VM instead of editing. Files are saved exactly as
entered; the original editor's `\#file` filename convention is replaced by
the explicit file path field. Load and exit do not prompt to save, so save
modified source first.

## Headless rendering

No window, audio device or desktop dependencies are required:

```sh
cargo run --release --no-default-features -- \
  --headless -c '^xp M d3r15&*' --frames 120 \
  --output output --wav output/sound.wav
```

This writes `frame-000000.ppm`, etc., and an optional signed 16-bit mono WAV
at 61,440 Hz. Each tick represents 1/60 second and contains 1,024 samples.
Existing numbered output files are overwritten. Exported sound is full scale,
independent of the desktop volume control.

With FFmpeg installed:

```sh
ffmpeg -framerate 60 -i output/frame-%06d.ppm -i output/sound.wav \
  -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest output/demo.mp4
```

Run `cargo run -- --help` for all options.

## Language and VM

IBNIZ uses 32-bit signed 16.16 fixed-point values, hexadecimal literals,
cyclic data/return stacks, and a 1,048,576-word shared address space.
Video uses two 256x256 VVUU.YYYY pages, converted through paired YUY2
chrominance to RGB. Audio uses the low 16 bits as unsigned PCM internally.

```text
^xp                 XOR texture: Y XOR X, discard T
d3r15&*             "42 melody", in the audio context
^xp M d3r15&*       separate video and audio programs
```

The compiler/interpreter implements the full documented instruction set,
including `z`, loops, subroutines, return stacks, arbitrary memory access,
user input, data segments, and automatic TYX/T video mode detection.
See the [original language manual](https://github.com/viznut/IBNIZ/blob/master/src/ibniz.txt)
for the complete instruction reference.

```rust
use ibniz::{Program, Vm};

let program = Program::compile("^xp M d3r15&*").expect("valid IBNIZ source");
let mut vm = Vm::new(program);
let frame = vm.render_frame(4_000_000);
assert_eq!(frame.rgb.len(), 256 * 256 * 3);
assert_eq!(frame.audio.len(), 1024);
```

`Vm::step` executes individual instructions. `memory_mut`, `set_input`,
`replace_program`, `reset` and `status` support embedding and debugging.

### Compatibility

Reference: upstream commit `3ba06ae5f2cf5765c0e06874a093f767054b6359`
(the 1.18 interpreter). Tests include 168 C-generated snapshots covering
arithmetic, stack and control operations, comparing the stack top, SP and
the complete memory hash.

This is not a cycle-exact SDL frontend clone:

- Scheduling uses deterministic 60 Hz ticks with bounded work; desktop sound
  is resampled to the output device's rate. Native code generation, legacy
  keystroke recording/playback, and the original `-M` stream format are not
  implemented. Use PPM/WAV exports instead.
- Automatic audio/video role reversal is not implemented; with `M`, put
  video code first and audio code second.
- Flat conditional/subroutine skip points and the C interpreter's actual
  signed `l` shift behavior are preserved, including their quirks.
- Whitespace separates literals as documented (the C parser accidentally
  concatenates digits across whitespace). Unknown characters are diagnosed
  rather than silently executed as no-ops.
- Cyclic data reads and memory initialization work at all bit lengths,
  including 32-bit words; undefined C shifts and out-of-range reads are not
  reproduced. Negative jump values wrap as unsigned 32-bit addresses.
- Floating-point transcendental results and YUV-to-RGB rounding may vary
  slightly by platform. No unsafe Rust is used in the VM.

The bundled `.ib` examples come from the upstream project. `oneliners.ib`
is a commented collection; uncomment a line to run it. Its original Latin-1
nonbreaking space has been converted to UTF-8.

## Development

```sh
cargo test
cargo test --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

The core and headless build have no third-party dependencies. Desktop tests
exercise the editor and rendering UI without opening a native window.

## License

[Zlib](LICENSE), retaining the original IBNIZ copyright and notice.
Original IBNIZ by Ville-Matias Heikkila (viznut); Rust implementation and
frontend are an altered version. Upstream examples retain the same license.
