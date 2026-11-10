# Improvements

Recommended priority: example browser, unsaved-change protection, then
recording controls. The remaining improvements are not ranked.

## Example browser

- [x] Browse and load bundled examples inside the application.
- [x] Display descriptions, contributor credits, and source links.
- [x] Categorize examples into fractals, textures, music, and other groups.

## Editor improvements

- [x] Add syntax highlighting.
- [x] Display compile errors inline at the relevant source location.
- [x] Confirm before loading another file or exiting with unsaved changes.

## Recording controls

- [x] Export video and audio directly from the GUI.
- [x] Optionally use FFmpeg to create MP4 recordings.

## VM debugger

- [x] Support single-step execution.
- [x] Inspect data and return stacks.
- [x] Add a memory viewer.
- [x] Highlight the current instruction in the source editor.

## Playback improvements

- [x] Improve handling of computationally expensive programs.
- [x] Improve audio underrun handling.
- [x] Expose adjustable quality and instruction budgets in the GUI.

## Video optimization plan

The headless audit ran all 195 `.ib` examples for 1,800 ticks each (30 simulated
seconds), using the default 4,000,000 instructions per tick and default input.
Eight examples exhausted their budget before a video page flip:

| Example | Incomplete video ticks | Rate |
|---------|------------------------|------|
| `summerlab-muon.ib` | 1,715 | 95.3% |
| `mandelbrot.ib` | 1,539 | 85.5% |
| `animated-mandelbrot-set.ib` | 1,450 | 80.6% |
| `julia.ib` | 1,293 | 71.8% |
| `interactive-julia-sets.ib` | 1,200 | 66.7% |
| `julia-sets-morpher.ib` | 1,064 | 59.1% |
| `summerlab-snotty-ice-cream-electrode.ib` | 900 | 50.0% |
| `ibnizdemo.ib` | 224 | 12.4% |

These are instruction-budget misses, not confirmed buffer corruption. Faster
execution alone will not remove misses at the same instruction-count limit.
The other 187 examples had no incomplete video ticks within the test window;
this was not visual inspection or interactive mouse testing.

Five-second follow-up runs eliminated misses for seven examples at 64,000,000
instructions per tick; `summerlab-muon.ib` needed 128,000,000. Several then
exceeded the 16.7 ms real-time deadline. `ibnizdemo.ib` and
`summerlab-snotty-ice-cream-electrode.ib` stayed below that deadline on the audit
machine and are promising early targets. Results are hardware-dependent.

Implement in the following order:

### 1. Establish a repeatable baseline

- [x] Extend `examples/playback_bench.rs` to cover all `.ib` examples.
- [x] Measure VM execution, RGB conversion, and desktop texture upload separately.
- [x] Measure instructions required to complete video pages, with a bounded cap
      for non-producing programs.
- [x] Report median/p95 tick duration, incomplete video ticks, and audio misses.
- [x] Use the eight affected examples as stress cases and simple textures as
      regression controls; include mouse-input scenarios for interactive programs.

Implemented in `examples/playback_bench.rs`; see the README's **Playback
benchmark** section for commands and metric definitions. The native
`--desktop` mode measures synchronized OpenGL texture uploads separately from
preparation, not just egui command queueing. Headless output leaves those
measurements empty. Page probes use a separate VM, preserve capped work between
ticks, and report unfinished work rather than pretending it completed.

The initial instrumented release run covered all 195 files and 199 input
scenarios (300 measured ticks, no warmup). Every default-input video-miss count
matched the earlier 300-tick audit. On the Apple M4 Max audit machine, affected
examples had RGB-conversion p95 times around 0.13 ms versus execution p95 times
of 4.9-9.8 ms at the default budget. A native run also measured 300 actual
uploads per control after 60 warmup ticks. These measurements support
prioritizing VM execution; they are not portable performance thresholds.

### 2. Reduce conversion and display overhead

Previously every completed tick allocated and converted RGB in `src/vm.rs`,
even when the preview refresh policy skipped texture upload in
`src/desktop/mod.rs`.

- [x] Add an internal conversion-into-reusable-buffer path while preserving the
      existing public API.
- [x] Convert only when preview or recording needs an image; preserve debugger,
      headless export, and recording behavior.
- [x] Benchmark vectorized conversion only if profiling shows conversion remains
      significant; require byte-identical RGB results.

Desktop playback now uses `FrameRender::advance_tick` and a reusable
`Vm::rgb_into` destination. Conversion runs once when a preview or recording
needs the image; skipped previews do not convert. Recording still captures
every VM tick, and debugger steps always refresh their image without a second
conversion at tick completion. Existing full-frame APIs and headless exports
remain unchanged.

A release-mode headless pipeline comparison (600 measured ticks after 60
warmup ticks, seven control/stress programs) reduced conversions from 600 to
150 at a simulated 15 fps preview cadence, with identical instruction and
video-miss counts. This measures conversion policy, not native UI throughput.
Scalar conversion remains around 0.13 ms per image on the audit machine, well
below the 16.7 ms tick deadline; SIMD benchmarking is deferred rather than
adding architecture-specific complexity for a small absolute cost. Conversion
is checked byte-for-byte against the original scalar arithmetic.

Do not treat a missing page flip as proof that the visible image is unchanged:
programs can write directly into visible memory.

### 3. Optimize profiled VM hot paths

- [x] Profile `src/vm.rs` before selecting execution optimizations.
- [x] Expand existing instruction fusion for frequently executed arithmetic and
      stack sequences where measurements justify it.
- [x] Evaluate predecoding fusion opportunities instead of repeatedly matching
      instruction patterns.
- [x] Investigate loop dispatch, stack access, and expensive arithmetic only
      where profiles identify bottlenecks.

Native CPU sampling located the hot work in the interpreter loop. A separate
single-instruction trace of three default-input ticks per stress/control case
(4,000,000 instructions per tick) identified literal memory reads/stores,
literal stack picks, and duplicate/multiply sequences as frequent operations.
Literal reads alone occurred at roughly 9-22% of executed instruction positions
in several affected programs; `mandelbrot.ib` also spent about 15% of its
instructions on `L`. These are workload-specific counts, not time percentages.

The compiler now records fusion hints alongside the original instructions.
Rendering fuses literal `@`, `!`, and `)` pairs and `d*`, and uses a predecoded
operand for the existing `d literal & *` fusion. Source instruction addresses
are unchanged: jumps into sequences and debugger steps still execute the
original instructions. Fusions run only when the slice and tick budgets can
accommodate every instruction, including temporary writes and audio-position
effects. Live recompilation refreshes the hints and execution flags.

The extended dispatch is selected once per render slice only for programs
containing eligible sequences. Simple programs retain the smaller dispatch.
Broader multiply/stack fusions and an out-of-line memory helper were measured
and dropped: their gains did not justify their regressions or extra complexity.
No loops are skipped, arithmetic approximated, or instruction budgets raised.

Alternating release comparisons against commit `792d669` used 60 warmup ticks
and 300 measured ticks, with default input and the original tick budget.
Two paired runs of the retained implementation measured the following
reductions in median execution time (mean of the two per-run medians):

| Stress example | Execution time reduction |
|----------------|--------------------------|
| `animated-mandelbrot-set.ib` | 14.7% |
| `ibnizdemo.ib` | 2.0% |
| `interactive-julia-sets.ib` | 10.6% |
| `julia-sets-morpher.ib` | 16.3% |
| `julia.ib` | 23.5% |
| `mandelbrot.ib` | 7.1% |
| `summerlab-muon.ib` | 5.6% |
| `summerlab-snotty-ice-cream-electrode.ib` | 10.4% |

These are hardware/compiler-dependent tradeoffs, not universal speedups.
The four simple controls were within about -3% to +1% in that comparison.
Targeted repeats also found roughly 6-10% slower execution in the
`d7*xr/31a/` family (`summerlab-radiation.ib`,
`summerlab-four-rotations.ib`, and Summerlab lines 254/255); their absolute
median increases were approximately 0.16-0.25 ms. Keep these cases in future
code-generation/regression comparisons rather than assuming every program
benefits from the new interpreter layout.

All deterministic baseline fields matched across 195 examples and 199 input
scenarios. Differential tests compare RGB, audio, complete memory, registers,
and instruction counts against single-instruction execution, including cyclic
stack boundaries, aliased literal accesses, partial budgets, control-flow
entry inside sequences, and live recompilation during a partial tick.
Instruction-budget video misses remain unchanged; faster execution does not
remove the instruction-count limit.

Preserve instruction counts, intermediate memory writes, cyclic-stack behavior,
single-step debugging, and instruction-bounded rendering. Do not assume pixels
are independent: programs can access shared memory.

### 4. Tune instruction budgets separately

- [x] After execution improvements, evaluate an opt-in higher-budget playback
      mode while retaining bounded work slices and responsive controls.
- [x] Distinguish instruction-budget misses from wall-clock deadline misses in
      performance comparisons.
- [x] Confirm audio scheduling and recording semantics remain unchanged.

The Playback menu now offers Default (4M), Complex (64M), and Very complex
(128M) presets alongside the existing custom budget field. Selecting a preset
only changes the next tick's numeric budget. Partial ticks retain their
original budgets, controls and work slices remain unchanged, and no automatic
adaptation or VM reset occurs. Numeric budgets continue to use the existing
NSM session format. Tick diagnostics distinguish the actual current/last
budget from the configured next budget, and cumulative video-budget misses
are shown separately from late ticks and audio-budget misses.

Release-mode headless comparisons on the audit machine used the optimized
VM, default input, 60 warmup ticks and 300 measured ticks per stress case.
Each cell below counts measured ticks, not dropped device frames; p95 is total
headless tick time. No measured audio-budget misses occurred at any preset.

| Example | Video misses 4M | Video misses 64M | Video misses 128M | Late ticks 64M / 128M | p95 ms 64M / 128M |
|---------|-----------------|------------------|-------------------|-----------------------|-------------------|
| `animated-mandelbrot-set.ib` | 240 | 0 | 0 | 300 / 300 | 59.5 / 63.4 |
| `ibnizdemo.ib` | 52 | 0 | 0 | 0 / 0 | 10.6 / 10.7 |
| `interactive-julia-sets.ib` | 200 | 0 | 0 | 300 / 300 | 19.4 / 19.6 |
| `julia-sets-morpher.ib` | 174 | 0 | 0 | 102 / 123 | 22.9 / 23.5 |
| `julia.ib` | 215 | 0 | 0 | 300 / 300 | 35.4 / 38.2 |
| `mandelbrot.ib` | 247 | 0 | 0 | 300 / 300 | 75.0 / 76.5 |
| `summerlab-muon.ib` | 285 | 150 | 0 | 300 / 300 | 72.9 / 95.0 |
| `summerlab-snotty-ice-cream-electrode.ib` | 150 | 0 | 0 | 0 / 0 | 9.7 / 9.7 |

At 4M none of these measured ticks exceeded 16.7 ms, despite frequent video
budget misses. At 64M seven examples completed every measured video page;
128M also eliminated the muon misses, but neither higher preset guarantees
real-time playback. Start with 64M for `ibnizdemo.ib` and
`summerlab-snotty-ice-cream-electrode.ib`; the heavier examples still need
substantial execution speedups. Higher-cap runs are separate workloads because
budget-limited ticks advance time differently; this is not a same-image
speedup comparison or a guarantee about interactive inputs or device xruns.

Scheduling remains the same: one quarter of the selected instruction budget
is reserved for the audio phase, each completed tick supplies 1024 samples at
61440 Hz, and recording captures one image and its audio per VM tick,
regardless of preview refresh. Preset/partial-tick tests verify no reset or
budget replacement mid-tick. Recording comparisons across all three presets
match reference PPM frames and unscaled WAV samples, including pause/resume
and compile-error suspension. See the README for reproducible budget commands.

Raising the budget is not itself an execution optimization or a guarantee of
60 Hz playback. Heavy fractals and `summerlab-muon.ib` need substantial speedups.

### 5. Validate each optimization

- [x] Compare optimized and single-instruction execution: RGB, audio, memory,
      registers, and instruction counts, including budget and slice boundaries.
- [x] Run existing reference, slicing, debugger, and recording tests.
- [x] Repeat the 195-example, 1,800-tick audit and compare against the baseline.
- [ ] Confirm simple examples do not regress and complex examples improve in
      measured execution time, not merely through a larger instruction budget.

Validation of `f2a82a0` passed the desktop and headless Rust suites, the
release-mode VM differential and C-reference tests, and playback-benchmark
tests in both feature configurations. Both normally ignored FFmpeg tests
were explicitly run with installed FFmpeg/ffprobe and passed, covering MP4
encoding and encoder cancellation with source-media retention. Clippy passed
with warnings denied for desktop/all features and headless targets, and
formatting checks passed.

The differential suite compares full memory, RGB, audio, VM status/registers,
and logical instruction counts against single-instruction rendering. Cases
include tick/slice boundaries, stack wraparound, aliased memory, control-flow
entry inside fused sequences, partial-tick recompilation, and all 195 bundled
programs with default, center, and button/coordinate input. Desktop tests
also cover lazy conversion, debugger stepping, preview throttling, budget
presets, NSM persistence, and full-rate recording.

The full audit reran all 195 programs for 1,800 measured ticks each (351,000
ticks total), with no warmup, default input, and the original 4,000,000
instruction budget. Every example's video-miss, audio-miss, and budget-hit
count matched the original 30-second audit exactly. The same eight examples
had 9,385 incomplete video ticks in total; the remaining 187 had none.
There were no audio-budget misses or terminated programs. This confirms
unchanged budget-limited behavior, not elimination of the eight examples'
instruction limits.

The run used Rust 1.99.1 / LLVM 23.1.3 on an Apple M4 Max. All examples' p95
headless tick times were below 16.7 ms (maximum 9.219 ms). There was one
wall-clock outlier: `mandelbrot.ib` took 25.077 ms on one tick, versus zero
late ticks in the original audit. Deadline counts include scheduling noise
and are reported separately from the identical deterministic budget counts;
timings collected on different dates are not used to establish speedups.

**The no-performance-regression gate remains unmet.** Three alternating
baseline/optimized release pairs compared commit `792d669` with `f2a82a0`
on the audit machine. All used default input, 60 warmup ticks, 300 measured
ticks, and the same 4,000,000-instruction tick budget. Video/audio misses,
budget hits and mean instruction counts matched exactly in all 16 selected
stress/control/regression cases. The eight stress cases improved by
approximately 0.9-20.9% in median execution time (median of three per-run
medians), so those gains do not come from increasing the instruction budget.
However, the following slowdowns were reproduced:

| Example | Baseline median ms | Optimized median ms | Slowdown |
|---------|--------------------|---------------------|----------|
| `tv-noise.ib` | 0.463 | 0.482 | 4.0% |
| `xor-texture.ib` | 0.611 | 0.636 | 4.2% |
| `summerlab-radiation.ib` | 1.606 | 1.774 | 10.4% |
| `summerlab-four-rotations.ib` | 2.807 | 2.980 | 6.2% |
| `summerlab-summerlab-line-254.ib` | 3.402 | 3.663 | 7.7% |
| `summerlab-summerlab-line-255.ib` | 3.749 | 3.973 | 5.9% |

`plasma.ib` was essentially unchanged (about 0.1% slower), while
`mul-texture-zoomer.ib` improved by about 0.8%. These hardware/compiler-specific
measurements are evidence for follow-up interpreter/code-generation work,
not portable timing assertions for CI. Do not mark the final checkbox complete
until the reproduced regressions have been addressed and paired comparisons
repeated.

Start with baseline instrumentation and reusable/lazy RGB conversion, then use
profiles to guide VM fusion. Defer JIT compilation, GPU execution, and threading
until measurements justify their complexity.

## Distribution packages

- [x] Provide ready-to-install Linux packages.
- [x] Create a macOS application bundle.
- [x] Automate cross-platform builds.
