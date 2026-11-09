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

- [ ] Profile `src/vm.rs` before selecting execution optimizations.
- [ ] Expand existing instruction fusion for frequently executed arithmetic and
      stack sequences where measurements justify it.
- [ ] Evaluate predecoding fusion opportunities instead of repeatedly matching
      instruction patterns.
- [ ] Investigate loop dispatch, stack access, and expensive arithmetic only
      where profiles identify bottlenecks.

Preserve instruction counts, intermediate memory writes, cyclic-stack behavior,
single-step debugging, and instruction-bounded rendering. Do not assume pixels
are independent: programs can access shared memory.

### 4. Tune instruction budgets separately

- [ ] After execution improvements, evaluate an opt-in higher-budget playback
      mode while retaining bounded work slices and responsive controls.
- [ ] Distinguish instruction-budget misses from wall-clock deadline misses in
      performance comparisons.
- [ ] Confirm audio scheduling and recording semantics remain unchanged.

Raising the budget is not itself an execution optimization or a guarantee of
60 Hz playback. Heavy fractals and `summerlab-muon.ib` need substantial speedups.

### 5. Validate each optimization

- [ ] Compare optimized and single-instruction execution: RGB, audio, memory,
      registers, and instruction counts, including budget and slice boundaries.
- [ ] Run existing reference, slicing, debugger, and recording tests.
- [ ] Repeat the 195-example, 1,800-tick audit and compare against the baseline.
- [ ] Confirm simple examples do not regress and complex examples improve in
      measured execution time, not merely through a larger instruction budget.

Start with baseline instrumentation and reusable/lazy RGB conversion, then use
profiles to guide VM fusion. Defer JIT compilation, GPU execution, and threading
until measurements justify their complexity.

## Distribution packages

- [x] Provide ready-to-install Linux packages.
- [x] Create a macOS application bundle.
- [x] Automate cross-platform builds.
