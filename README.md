# IBNIZ Rust

A standalone Rust implementation of [IBNIZ](https://github.com/viznut/IBNIZ),
Ville-Matias Heikkila's compact audiovisual virtual machine. Includes a live
code editor, realtime video and sound, a reusable VM library, and headless
PPM/WAV rendering. This is an **altered implementation**, not the original C
distribution.

## Installers

Pushed version tags produce installers on the repository's **GitHub Releases**
page, together with `SHA256SUMS`:

| Platform | Installer | Compatibility |
|----------|-----------|---------------|
| Debian / Ubuntu | `.deb` | x86-64, Debian 12+ / Ubuntu 24.04+ |
| Fedora | `.rpm` | x86-64, Fedora 43+ |
| macOS | `.dmg` containing `IBNIZ Rust.app` | Separate Intel (`x86_64`) and Apple Silicon (`arm64`) builds, macOS 11+ |
| Windows | `.msi` | x86-64, Windows 10/11 |

Install Linux packages with `sudo apt install ./ibniz-*.deb` or
`sudo dnf install ./ibniz-*.rpm`. On macOS, open the disk image and drag the app
into **Applications**. On Windows, run the MSI; it adds a Start menu shortcut,
an uninstall entry and `ibniz` on `PATH` (open a new terminal after installing).
The Windows runtime is linked statically; no separate Visual C++ runtime
installation is required.

These are **unsigned distribution builds**: the macOS app is only ad-hoc signed,
not Developer ID signed or notarized, and Windows installers have no publisher
signature. Gatekeeper/SmartScreen may require explicit approval. Verify the
download source and checksum before approving installation.
When launching from Finder, choose writable absolute file/recording paths in
the editor rather than relying on its working directory.
FFmpeg is **not bundled**; install it separately for optional MP4 conversion.

### Publishing a release

`.github/workflows/installers.yml` runs on tag pushes. Update `[package].version`
in `Cargo.toml`, refresh `Cargo.lock` with Cargo, and commit those changes before
creating the matching tag:

```sh
# After committing the version bump and updated Cargo.lock:
git tag v0.2.0
git push origin v0.2.0
```

Tags may use `v0.2.0` or `0.2.0`; their version must match Cargo exactly.
SemVer prereleases such as `v0.2.0-rc.1` are marked as GitHub prereleases.
Invalid tags or mismatched versions fail before building installers.
The common installer version limit is `255.255.65535`. Windows MSI compares only
the numeric version core, so prereleases/build metadata with the same core can
replace each other; Debian/RPM prereleases sort before the final release.

All five builds must succeed before publication. Linux packages and Windows
MSIs are installed and checked on their runners; macOS disk images are verified
and their bundled executable checked. Assets are also retained as workflow
artifacts. A new release stays a draft until all installers and checksums have
uploaded; rerunning the workflow refreshes its assets. Only the publishing job
gets release-write permissions. No signing secrets are required.

Packaging uses Python 3.11+ and native `dpkg-deb`/`dpkg-shlibdeps`, `rpmbuild`,
macOS `sips`/`iconutil`/`codesign`/`hdiutil`, and WiX 5.0.2 with its matching UI
extension. For local packaging after building the native release binary:

```sh
python3 -m unittest discover -s packaging -p 'test_*.py'
python3 packaging/package.py build --tag v0.1.0 --format deb \
  --binary target/release/ibniz
```

Use a matching tag/version and the format appropriate to your host (`deb`,
`rpm`, `dmg`, or `msi`; Windows uses `ibniz.exe`). macOS takes
`--arch x86_64` or `--arch arm64`. Installers are written to `target/installers/`.

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

**Audio > DC blocker** toggles a 20 Hz high-pass filter for speaker playback.
It is **enabled by default** and removes sustained DC offset, with some
attenuation of very low bass. The filter runs at the output device's sample
rate and retains its state between audio callbacks, including during silence.
Uncheck it for unfiltered playback. This setting does not change the VM or
headless WAV exports (which have a separate `--dc-blocker` flag), and returns
to enabled when the application restarts outside an NSM session.
It is not a limiter or a guarantee of speaker or hearing protection; keep the
volume low.

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
| F3 | Step one VM instruction while paused, with no active/finishing recording |
| F12 | Show / hide help (shortcuts and instruction reference) |
| Tab | Show / hide editor |
| Ctrl/Cmd+S | Save to the file path above the editor |
| Ctrl+Up / Ctrl+Down | Increment / decrement the hex digit under the cursor, with carry |
| Esc | Cancel confirmation, close help, the example browser or debugger, otherwise exit |
| Load / Save | Open / save the specified UTF-8 `.ib` file |
| Examples | Browse, search and load bundled programs |
| Debugger | Inspect stacks/memory and single-step execution |

The editor supports platform-standard selection, clipboard, word navigation
and undo/redo. Theme-aware syntax colors distinguish hexadecimal numbers,
comments, control flow, stack/media instructions and data-section modes.
Changes recompile immediately **without resetting VM state**.
Compile errors underline the offending character: hover it for details, or
read the line/column diagnostic beneath the source. Columns count Unicode
characters, not UTF-8 bytes. Errors suspend playback until corrected; resetting
uses the last successfully compiled program. A `T` instruction stops execution;
use F2 and F1 to start it again.

The status bar shows stack pointers, video mode, time, instruction count and
audio health. Slow programs are bounded by `--max-instructions` per tick
(default 4,000,000); budget exhaustion is reported instead of freezing the UI.
Playback slows down if your machine cannot produce a tick in 1/60 second.

**Audio gaps** counts output callbacks without ready audio during playback,
including recovery buffering, with the total missing audio duration; the
indicator turns yellow after a gap or queue overflow. Startup, pauses and
compile-error suspension do not count.
One interruption may span several callbacks. This detects buffer starvation,
not clicks generated by the expression itself or every possible device glitch.
**Late ticks** counts missed 60 Hz deadlines (including UI/scheduling delays);
**Last tick** shows rendering work time against the 16.7 ms target.
**Audio budget misses** counts ticks where the VM could not generate all audio
within its instruction budget, even if the output queue did not run dry.
Counters persist across edits and pauses; F2 or loading a file resets them.

Mouse coordinates, buttons and modifiers feed `U`. Hide the editor to send
typed characters to the VM instead of editing. Files are saved exactly as
entered; the original editor's `\#file` filename convention is replaced by
the explicit file path field. File Load, bundled example loading and exit ask
before discarding unsaved changes. **Cancel** (or Escape) keeps your edits so
you can save first; **Discard** proceeds without saving. Failed file loads
preserve the current source and VM. Under NSM, ordinary window exit also
protects unsaved session settings, but manager-requested termination never
prompts.

### Bundled example browser

Click **Examples** to browse all 38 bundled programs, grouped into **Fractals**,
**Textures**, **Music**, **Demos**, and **Other**. Search matches titles,
filenames, descriptions, contributors and source collections, ignoring case.
Select an example to see its description, named contributor (where provided),
pinned source link, collection license and an optional read-only source preview.
Collection maintainers are not assumed to have authored individual programs.

**Load example** replaces the editor contents, resets the VM and audio queue,
and pauses playback. Press **Run** when ready; some examples flash rapidly or
produce loud noise. Unsaved edits require confirmation before replacement.
The suggested save path is `<example>-copy.ib` in the current directory, not the
bundled source file; change the path before saving if needed. Playback settings
are retained. An active recording stays armed but capture pauses until Run.
`oneliners.ib` is a commented collection: uncomment one program before running.

Programs, metadata and license notices are embedded in the executable, so the
browser works offline and outside the repository. Only opening external source
links requires a web browser/network connection.

### VM debugger

Click **Debugger**, pause with **F1**, then use **Step (F3)** or **F3** to
execute exactly one instruction. A number literal is one instruction, not one
character; the implicit end-of-program operation also counts as an instruction.
Steps are silent and refresh the video preview without resetting the VM.
They use the normal audio/video scheduler, preserving an in-progress tick;
frame time advances only when a full tick completes or reaches its instruction
budget. **Run** resumes from the same state. A `T` instruction still terminates
execution; use **F2** before stepping again.

The debugger shows the active context, instruction pointer, execution count
and next instruction. Choose **Active**, **Video** or **Audio** to inspect
either context's data and return stacks, with their SP/RSP and up to 64 slots
listed top-first. These are cyclic buffers, not bounded stacks: old slots stay
visible and the displayed row count is not a logical stack depth. Return-stack
values are raw addresses/counts, not necessarily fixed-point numbers.

The read-only memory viewer covers all **00000-FFFFF physical word addresses**
in 128-word pages. Enter a hexadecimal address, navigate pages, or jump to the
selected context's **SP/RSP**. Values appear as raw eight-digit hexadecimal;
hover for signed-integer and 16.16 interpretations.

While the debugger is open, the editor highlights the **next instruction**.
An EOF marker represents the implicit end instruction, and the line/column
label locates it even after comments or a data section. Invalid drafts suppress
the instruction highlight rather than pointing into the last valid program.
Stack and memory inspection remain available while running or recording,
but stepping requires paused playback, valid source and a fully finished
recording. Debugger window/view choices are not part of NSM session state.

### Playback tuning

The **Playback** menu changes settings live, without resetting the VM:

| Setting | Effect |
|---------|--------|
| Preview refresh: 15 / 30 / 60 fps | Limits display texture updates; defaults to 60 fps. VM execution and 60 fps recordings are unchanged. |
| Instructions/tick | Total VM instruction limit, initially `--max-instructions`. Changes apply to the next tick; higher limits allow more complex expressions but can slow playback. |
| Work slice: 1-12 ms | VM work per UI update, default 6 ms. Lower values favor editor responsiveness; higher values favor throughput. |
| Minimum audio buffer: 20-200 ms | Startup/recovery buffering, default 80 ms. Higher values tolerate jitter at the cost of playback latency. |

Expensive ticks resume across UI updates rather than blocking for their entire
instruction budget. Work-slice deadlines are checked every 1,024 instructions;
finishing a tick also converts its video image, so this is not a strict
wall-clock cap. The status bar shows in-progress instruction counts.
Pausing preserves partial work; resetting or loading a program discards it.
**Last tick** sums actual rendering work across slices, excluding time waiting
between UI updates.

Audio resampling happens outside the output-queue lock. Playback waits for real
samples instead of padding the queue with silence. After starvation, the buffer
target grows by 20 ms, up to 200 ms, before resuming; short 5 ms fades soften
the transition to and from silence. The status bar shows queued audio and the
current target. Pause/reset clears queued audio and restores the selected
minimum; changing the minimum sets the target immediately. The queue remains
capped at 250 ms, with discarded samples reported by the existing counter.

These changes tolerate short scheduling delays; they cannot make a program
that consistently produces audio too slowly play continuously. Sustained
overload still slows playback and can cause gaps. Buffering and fades affect
speaker playback only, not VM audio, headless output, or GUI recordings.

### GUI recording

Expand **Recording** above the editor, choose an output folder, and click
**Start recording**. This captures future VM ticks without resetting playback;
if paused, press **Run** to begin capture. **Stop recording** finalizes the take.
Each take gets a new subfolder containing `frame-000000.ppm`, etc., and
`sound.wav` (61,440 Hz, signed 16-bit mono). Audio is **full scale and unfiltered**,
independent of speaker volume, the playback DC blocker, and audio-device
availability. Keep playback volume low when opening recordings.

Run/pause and compile errors pause capture without inserting silence or
duplicate frames. Edits, resets and loaded programs appear in the same take.
VM termination stops recording; closing the window confirms unsaved changes
first, then waits for finalization with editing disabled.
Recordings use the VM's 60 Hz timeline, not wall-clock time: a slow expression
still exports one video frame per 1,024 audio samples. Instruction-budget
exhaustion can still produce incomplete video or stale audio, as in headless
exports.

Check **Create MP4** before starting to encode `recording.mp4` after Stop.
This uses **ffmpeg-sidecar**, as in hydra-rust, with an installed `ffmpeg` on
`PATH` (or alongside the executable); **nothing is downloaded automatically**.
FFmpeg must support `libx264` and AAC. MP4 uses H.264/yuv420p video at 60 fps
and AAC audio resampled to 48 kHz. PPM/WAV originals are always retained,
including when conversion fails; existing recordings are never overwritten.
Encoder diagnostics are retained in the take's `ffmpeg.log`.
An empty take creates only an empty WAV and skips MP4 conversion.

Disk writes and encoding run off the UI thread with a bounded eight-frame
queue. If the disk cannot keep up, recording stops with a visible error rather
than silently dropping frames or accumulating memory. Queued frames are
finalized. Write/size-limit and FFmpeg errors appear in the status bar.
PPM sequences consume roughly 11.3 MiB/s, plus audio; WAV recordings are limited
to about 9.7 hours by the standard WAV file-size limit.

### NSM sessions

The desktop supports [New/Non Session Manager (NSM)](https://new-session-manager.jackaudio.org/).
Install the executable on `PATH` (see Linux desktop integration above), then add
`ibniz` as a client in your session manager. NSM supplies `NSM_URL`; no extra
application flag is needed. Without it, standalone behavior is unchanged.
Invalid or unreachable managers produce a visible warning and fall back to
standalone mode. Headless exports ignore `NSM_URL`.

Each client uses the project directory assigned by NSM, containing a
self-contained `state.json`. Session saves preserve editor source (including
invalid drafts), the last valid program, run/pause and editor visibility,
volume, DC blocker, instruction budget, work slice, preview refresh and minimum
audio buffer. Loading starts the VM at **time zero** with memory/stacks and
diagnostic counters reset, rather than resuming an execution snapshot.
Invalid drafts remain visible and suspend playback until corrected.

NSM Save and **Ctrl/Cmd+S** write the session state atomically. The ordinary file
path is locked and file Load is disabled; bundled examples instead offer
**Import example**, keeping saves inside the session. Source and playback-setting
changes are reported to the manager as dirty/clean. Recordings default to
`<project>/recordings`; explicitly chosen external recording folders are exports.
Stop recording and wait for finalization/conversion before saving the session:
while a take is active or finishing, Save returns an explicit “not now” error.

NSM termination (`SIGTERM` on Unix) exits without confirmation or an implicit
save, even with unsaved edits. Queued recording frames are drained and WAV
headers finalized; optional MP4 conversion is skipped or cancelled so it does
not delay session shutdown. PPM/WAV originals remain available.

The client supports OSC over **UDP** and dirty-state reporting. Live session
switching and optional-GUI hiding are not advertised: the manager must restart
the client for another project. Audio remains CPAL-based; NSM support does not
add JACK ports or connection restoration.

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
WAV audio is streamed as each tick is rendered, so recording memory does not
grow with its duration. Recordings exceeding the standard WAV size limit
are rejected before rendering starts.

Add `--dc-blocker` alongside `--headless --wav FILE` to apply the same 20 Hz
high-pass filter to exported audio:

```sh
cargo run --release --no-default-features -- \
  --headless -c '^xp M d3r15&*' --frames 120 \
  --output output --wav output/sound.wav --dc-blocker
```

Unlike desktop playback, headless filtering is **off by default** so existing
exports remain unchanged. The filter runs at 61,440 Hz, retains its state
across ticks, and clips filtered peaks to the signed 16-bit range. It does
not change the VM or exported video, and does not increase recording memory
with duration.

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
`export::WavWriter` accepts a total sample count and writes successive chunks
without requiring a seekable output. Wrap file outputs in `std::io::BufWriter`;
call `finish` to verify the count and flush buffered output.

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

The [examples catalog](examples/README.md) includes the original five upstream
files plus 33 deduplicated programs from the manual, jibniz and IBNIZ-js
galleries, with source credits and licenses. `oneliners.ib` is a commented
collection; uncomment a line to run it. Its original Latin-1 nonbreaking
space has been converted to UTF-8.

## Development

```sh
cargo test
cargo test --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

To exercise GUI MP4 encoding against installed FFmpeg and ffprobe:

```sh
cargo test --bin ibniz ffmpeg_encodes_recorded_video_and_audio -- --ignored
```

The core and headless build have no third-party dependencies. Desktop tests
exercise the editor and rendering UI without opening a native window.

## License

[Zlib](LICENSE), retaining the original IBNIZ copyright and notice.
Original IBNIZ by Ville-Matias Heikkila (viznut); Rust implementation and
frontend are an altered version. Original upstream examples retain the same
license; additional gallery examples retain their source collection's
MIT or WTFPL license, documented in [examples/README.md](examples/README.md).
