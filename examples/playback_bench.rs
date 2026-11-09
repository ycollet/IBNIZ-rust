use std::{
    collections::VecDeque,
    error::Error,
    fs,
    hint::black_box,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use ibniz::{Frame, FrameProfile, Instruction, Program, Vm};

#[cfg(feature = "desktop")]
#[path = "playback_bench/desktop.rs"]
mod desktop;

const DEADLINE: Duration = Duration::from_nanos(1_000_000_000 / 60);
const STRESS: &[&str] = &[
    "animated-mandelbrot-set.ib",
    "ibnizdemo.ib",
    "interactive-julia-sets.ib",
    "julia-sets-morpher.ib",
    "julia.ib",
    "mandelbrot.ib",
    "summerlab-muon.ib",
    "summerlab-snotty-ice-cream-electrode.ib",
];
const CONTROLS: &[&str] = &[
    "tv-noise.ib",
    "plasma.ib",
    "xor-texture.ib",
    "mul-texture-zoomer.ib",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Default,
    Center,
    Sweep,
}

impl Input {
    fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Center => "center",
            Self::Sweep => "sweep",
        }
    }

    fn word(self, tick: usize) -> u32 {
        match self {
            Self::Default => 0,
            Self::Center => 0x8080,
            Self::Sweep => {
                let phase = (tick % 512) as u32;
                let x = if phase < 256 { phase } else { 511 - phase };
                let y = (tick % 256) as u32;
                let button = u32::from((tick / 60).is_multiple_of(2)) << 31;
                button | (y << 8) | x
            }
        }
    }
}

struct Case {
    name: String,
    program: Program,
    input: Input,
}

struct Options {
    frames: usize,
    warmup: usize,
    budget: usize,
    page_frames: usize,
    page_cap: usize,
    directory: PathBuf,
    suite: String,
    filter: String,
    input: String,
    desktop: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            frames: 300,
            warmup: 60,
            budget: 4_000_000,
            page_frames: 3,
            page_cap: 128_000_000,
            directory: Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"),
            suite: "all".into(),
            filter: String::new(),
            input: "all".into(),
            desktop: false,
        }
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn options(args: impl Iterator<Item = String>) -> Result<Option<Options>, Box<dyn Error>> {
    let mut options = Options::default();
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!(
                "Playback baseline (use --release; CSV on stdout, metadata on stderr).\n\
                 --frames N             Measured ticks per case (default: 300)\n\
                 --warmup N             Unmeasured ticks per case (default: 60)\n\
                 --max-instructions N   Instruction budget per tick (default: 4000000)\n\
                 --page-frames N        Separate page-probe ticks from reset (default: 3)\n\
                 --page-cap N           Instruction cap per probe tick (default: 128000000)\n\
                 --examples-dir PATH    Directory containing .ib files (default: bundled sources)\n\
                 --suite NAME           all, stress, or controls (default: all)\n\
                 --filter TEXT          Filename substring\n\
                 --input NAME           all, default, center, or sweep (default: all)\n\
                 --desktop              Native OpenGL upload measurement; requires desktop feature\n\
                 'all' adds center/sweep scenarios only to programs containing U.\n\
                 Page probes use a separate VM and never alter the measured-budget run."
            );
            return Ok(None);
        }
        if arg == "--desktop" {
            options.desktop = true;
            continue;
        }
        if !matches!(
            arg.as_str(),
            "--frames"
                | "--warmup"
                | "--max-instructions"
                | "--page-frames"
                | "--page-cap"
                | "--examples-dir"
                | "--suite"
                | "--filter"
                | "--input"
        ) {
            return Err(invalid(format!("Unknown argument: {arg}")).into());
        }
        let value = args
            .next()
            .ok_or_else(|| invalid(format!("Missing value for {arg}")))?;
        match arg.as_str() {
            "--frames" => options.frames = value.parse()?,
            "--warmup" => options.warmup = value.parse()?,
            "--max-instructions" => options.budget = value.parse()?,
            "--page-frames" => options.page_frames = value.parse()?,
            "--page-cap" => options.page_cap = value.parse()?,
            "--examples-dir" => options.directory = value.into(),
            "--suite" => options.suite = value,
            "--filter" => options.filter = value,
            "--input" => options.input = value,
            _ => unreachable!(),
        }
    }
    if options.frames == 0
        || options.page_frames == 0
        || options.budget < 4
        || options.page_cap < 4
        || options.warmup.checked_add(options.frames).is_none()
    {
        return Err(invalid(
            "frames/page-frames must be positive, budgets at least 4, and warmup + frames must fit usize",
        )
        .into());
    }
    if !matches!(options.suite.as_str(), "all" | "stress" | "controls") {
        return Err(invalid("--suite must be all, stress, or controls").into());
    }
    if !matches!(
        options.input.as_str(),
        "all" | "default" | "center" | "sweep"
    ) {
        return Err(invalid("--input must be all, default, center, or sweep").into());
    }
    Ok(Some(options))
}

fn cases(options: &Options) -> Result<VecDeque<Case>, Box<dyn Error>> {
    let mut paths = fs::read_dir(&options.directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut cases = VecDeque::new();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("ib") || !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| invalid(format!("Non-UTF-8 example filename: {}", path.display())))?;
        if !name.contains(&options.filter)
            || (options.suite == "stress" && !STRESS.contains(&name))
            || (options.suite == "controls" && !CONTROLS.contains(&name))
        {
            continue;
        }
        let program = Program::compile(&fs::read_to_string(&path)?)
            .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
        let interactive = program.instructions().contains(&Instruction::Op(b'U'));
        let inputs: &[Input] = match options.input.as_str() {
            "all" if interactive => &[Input::Default, Input::Center, Input::Sweep],
            "center" => &[Input::Center],
            "sweep" => &[Input::Sweep],
            _ => &[Input::Default],
        };
        for &input in inputs {
            cases.push_back(Case {
                name: name.into(),
                program: program.clone(),
                input,
            });
        }
    }
    if cases.is_empty() {
        return Err(invalid("No examples match the directory, suite, and filter").into());
    }
    Ok(cases)
}

#[derive(Default)]
struct Samples(Vec<Duration>);

impl Samples {
    fn summary(&mut self) -> String {
        if self.0.is_empty() {
            return ",,".into();
        }
        self.0.sort_unstable();
        let len = self.0.len();
        let ms = |index: usize| self.0[index].as_secs_f64() * 1000.0;
        format!(
            "{:.6},{:.6},{:.6}",
            ms(len / 2),
            ms(len - len / 20 - 1),
            ms(len - 1)
        )
    }
}

#[derive(Default)]
struct UploadTiming {
    prepare: Duration,
    upload: Duration,
}

trait Upload {
    fn upload(&mut self, rgb: &[u8]) -> Result<UploadTiming, Box<dyn Error>>;
}

#[derive(Default)]
struct Stats {
    tick: Samples,
    execution: Samples,
    rgb: Samples,
    prepare: Samples,
    upload: Samples,
    late: usize,
    audio_misses: usize,
    video_misses: usize,
    budget_hits: usize,
    stopped_ticks: usize,
    instructions: u128,
    pages: usize,
    capped: usize,
    stopped_probes: usize,
    page_instructions: u128,
    max_page_instructions: u128,
    pending_page_instructions: u128,
}

impl Stats {
    fn record(
        &mut self,
        frame: &Frame,
        profile: FrameProfile,
        status: &ibniz::VmStatus,
        elapsed: Duration,
        budget: usize,
    ) {
        self.tick.0.push(elapsed);
        self.execution.0.push(profile.execution);
        self.rgb.0.push(profile.rgb);
        self.late += usize::from(elapsed > DEADLINE);
        self.audio_misses +=
            usize::from(!status.stopped && status.audio_enabled && !frame.audio_complete);
        self.video_misses += usize::from(!status.stopped && !frame.video_complete);
        self.budget_hits += usize::from(frame.instructions >= budget);
        self.stopped_ticks += usize::from(status.stopped);
        self.instructions += frame.instructions as u128;
    }

    fn probe(&mut self, frame: &Frame, profile: FrameProfile, stopped: bool) {
        self.pending_page_instructions += profile.video_instructions as u128;
        if frame.video_complete {
            self.pages += 1;
            self.page_instructions += self.pending_page_instructions;
            self.max_page_instructions = self
                .max_page_instructions
                .max(self.pending_page_instructions);
            self.pending_page_instructions = 0;
        } else if stopped {
            self.stopped_probes += 1;
        } else {
            self.capped += 1;
        }
    }

    fn print(&mut self, case: &Case, options: &Options) {
        let mean_page = if self.pages == 0 {
            String::new()
        } else {
            (self.page_instructions / self.pages as u128).to_string()
        };
        let max_page = if self.pages == 0 {
            String::new()
        } else {
            self.max_page_instructions.to_string()
        };
        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv(&case.name),
            case.input.name(),
            options.frames,
            options.warmup,
            options.budget,
            self.tick.summary(),
            self.execution.summary(),
            self.rgb.summary(),
            self.prepare.summary(),
            self.upload.summary(),
            self.late,
            self.audio_misses,
            self.video_misses,
            self.budget_hits,
            self.stopped_ticks,
            self.instructions / options.frames as u128,
            options.page_frames,
            options.page_cap,
            self.pages,
            self.capped,
            self.stopped_probes,
            mean_page,
            max_page,
            self.pending_page_instructions,
            if options.desktop {
                "opengl-sync"
            } else {
                "headless"
            },
            self.upload.0.len(),
        );
    }
}

fn csv(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

struct Run {
    case: Case,
    vm: Vm,
    probe: Vm,
    tick: usize,
    probe_tick: usize,
    stats: Stats,
}

struct Runner {
    options: Options,
    cases: VecDeque<Case>,
    active: Option<Run>,
}

impl Runner {
    fn new(options: Options) -> Result<Self, Box<dyn Error>> {
        let cases = cases(&options)?;
        eprintln!(
            "ibniz={}, cases={}, frames={}, warmup={}, budget={}, page_frames={}, page_cap={}, desktop={}; {}-{}; debug_assertions={}",
            env!("CARGO_PKG_VERSION"),
            cases.len(),
            options.frames,
            options.warmup,
            options.budget,
            options.page_frames,
            options.page_cap,
            options.desktop,
            std::env::consts::OS,
            std::env::consts::ARCH,
            cfg!(debug_assertions),
        );
        if cfg!(debug_assertions) {
            eprintln!("warning: use --release for meaningful performance measurements");
        }
        println!(
            "case,input,frames,warmup,budget,\
             median_ms,p95_ms,worst_ms,\
             execution_median_ms,execution_p95_ms,execution_worst_ms,\
             rgb_median_ms,rgb_p95_ms,rgb_worst_ms,\
             texture_prepare_median_ms,texture_prepare_p95_ms,texture_prepare_worst_ms,\
             upload_median_ms,upload_p95_ms,upload_worst_ms,\
             late_ticks,audio_misses,video_misses,budget_hits,stopped_ticks,mean_instructions,\
             page_probe_ticks,page_cap,completed_pages,capped_page_ticks,stopped_page_ticks,\
             mean_completed_page_video_instructions,max_completed_page_video_instructions,\
             unfinished_page_video_instructions,mode,upload_samples"
        );
        Ok(Self {
            options,
            cases,
            active: None,
        })
    }

    /// Execute one bounded tick or probe. Returns true when every case is done.
    fn step(&mut self, mut upload: Option<&mut dyn Upload>) -> Result<bool, Box<dyn Error>> {
        if self.active.is_none() {
            let Some(case) = self.cases.pop_front() else {
                return Ok(true);
            };
            self.active = Some(Run {
                vm: Vm::new(case.program.clone()),
                probe: Vm::new(case.program.clone()),
                case,
                tick: 0,
                probe_tick: 0,
                stats: Stats::default(),
            });
        }
        let run = self.active.as_mut().expect("active benchmark case");
        if run.tick < self.options.warmup + self.options.frames {
            run.vm.set_input(run.case.input.word(run.tick));
            let started = Instant::now();
            let (frame, profile) = black_box(run.vm.render_frame_profiled(self.options.budget));
            let uploaded = if let Some(upload) = upload.as_mut() {
                Some(upload.upload(&frame.rgb)?)
            } else {
                None
            };
            let elapsed = started.elapsed();
            if run.tick >= self.options.warmup {
                run.stats.record(
                    &frame,
                    profile,
                    &run.vm.status(),
                    elapsed,
                    self.options.budget,
                );
                if let Some(timing) = uploaded {
                    run.stats.prepare.0.push(timing.prepare);
                    run.stats.upload.0.push(timing.upload);
                }
            }
            run.tick += 1;
        } else if run.probe_tick < self.options.page_frames {
            run.probe.set_input(run.case.input.word(run.probe_tick));
            let (frame, profile) =
                black_box(run.probe.render_frame_profiled(self.options.page_cap));
            run.stats.probe(&frame, profile, run.probe.status().stopped);
            run.probe_tick += 1;
        } else {
            run.stats.print(&run.case, &self.options);
            self.active = None;
        }
        Ok(false)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Some(options) = options(std::env::args().skip(1))? else {
        return Ok(());
    };
    if options.desktop {
        #[cfg(feature = "desktop")]
        return desktop::run(Runner::new(options)?);
        #[cfg(not(feature = "desktop"))]
        return Err(invalid("--desktop requires rebuilding without --no-default-features").into());
    }
    let mut runner = Runner::new(options)?;
    while !runner.step(None)? {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_compile_and_include_stress_controls_and_empty_programs() {
        let cases = cases(&Options::default()).unwrap();
        let names: std::collections::BTreeSet<_> =
            cases.iter().map(|case| case.name.as_str()).collect();
        let files = fs::read_dir(Options::default().directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("ib"))
            .count();
        assert_eq!(names.len(), files);
        for name in STRESS.iter().chain(CONTROLS) {
            assert!(names.contains(name), "{name}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.program.instructions() == [Instruction::End])
        );
        let interactive: Vec<_> = cases
            .iter()
            .filter(|case| case.name == "interactive-julia-sets.ib")
            .collect();
        assert_eq!(interactive.len(), 3);
        assert_eq!(interactive[0].input, Input::Default);
        assert_eq!(interactive[1].input, Input::Center);
        assert_eq!(interactive[2].input, Input::Sweep);
    }

    #[test]
    fn input_words_follow_desktop_packing_and_sweep_is_repeatable() {
        assert_eq!(Input::Center.word(0), 0x8080);
        assert_eq!(Input::Sweep.word(0), 0x80000000);
        assert_eq!(Input::Sweep.word(60), 0x3c3c);
        assert_eq!(Input::Sweep.word(256) & 0xffff, 0x00ff);
        assert_eq!(Input::Sweep.word(511) & 0xffff, 0xff00);
        assert_eq!(Input::Sweep.word(512) & 0xffff, 0);
    }

    #[test]
    fn page_probe_distinguishes_completion_cap_and_termination() {
        for (source, expected) in [("", (1, 0, 0)), ("[1]", (0, 1, 0)), ("T", (0, 0, 1))] {
            let mut vm = Vm::new(Program::compile(source).unwrap());
            let (frame, profile) = vm.render_frame_profiled(1_000_000);
            let mut stats = Stats::default();
            stats.probe(&frame, profile, vm.status().stopped);
            assert_eq!(
                (stats.pages, stats.capped, stats.stopped_probes),
                expected,
                "{source}"
            );
            assert_eq!(
                profile.audio_instructions + profile.video_instructions,
                frame.instructions
            );
            if stats.pages > 0 {
                assert!(stats.page_instructions > 0);
            }
        }
    }

    #[test]
    fn stopped_ticks_are_not_reported_as_media_budget_misses() {
        let mut vm = Vm::new(Program::compile("T").unwrap());
        let (frame, profile) = vm.render_frame_profiled(100);
        let mut stats = Stats::default();
        stats.record(&frame, profile, &vm.status(), Duration::ZERO, 100);
        assert_eq!(
            (stats.audio_misses, stats.video_misses, stats.stopped_ticks),
            (0, 0, 1)
        );
    }

    #[test]
    fn page_probe_accumulates_work_across_capped_ticks() {
        let mut vm = Vm::new(Program::compile("").unwrap());
        let mut stats = Stats::default();
        let mut video_instructions = 0_u128;
        for _ in 0..100 {
            let (frame, profile) = vm.render_frame_profiled(4096);
            video_instructions += profile.video_instructions as u128;
            stats.probe(&frame, profile, vm.status().stopped);
            if frame.video_complete {
                break;
            }
        }
        assert_eq!(stats.pages, 1);
        assert!(stats.capped > 0);
        assert_eq!(stats.page_instructions, video_instructions);
        assert_eq!(stats.pending_page_instructions, 0);
    }

    #[test]
    fn validates_options_and_filters_without_silent_empty_runs() {
        for args in [
            vec!["--frames", "0"],
            vec!["--page-frames", "0"],
            vec!["--page-cap", "3"],
            vec!["--suite", "unknown"],
            vec!["--input", "unknown"],
            vec!["--filter"],
        ] {
            assert!(options(args.into_iter().map(String::from)).is_err());
        }
        let options = Options {
            filter: "no-such-example".into(),
            ..Options::default()
        };
        assert!(cases(&options).is_err());
        let options = Options {
            suite: "stress".into(),
            input: "default".into(),
            ..Options::default()
        };
        assert_eq!(cases(&options).unwrap().len(), STRESS.len());
        let options = Options {
            suite: "controls".into(),
            ..Options::default()
        };
        assert_eq!(cases(&options).unwrap().len(), CONTROLS.len());
    }
}
