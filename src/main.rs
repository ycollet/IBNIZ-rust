use std::{
    error::Error,
    fs::{self, File},
    io::{self, BufWriter},
    path::PathBuf,
};

use ibniz::{Program, Vm, export};

mod dc_blocker;
use dc_blocker::DcBlocker;

#[cfg(feature = "desktop")]
mod desktop;

struct Options {
    source: String,
    path: Option<PathBuf>,
    autorun: bool,
    audio: bool,
    headless: bool,
    frames: usize,
    output: PathBuf,
    wav: Option<PathBuf>,
    wav_dc_blocker: bool,
    budget: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ibniz: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut options = Options {
        source: "^xp M d3r15&*".into(),
        path: None,
        autorun: true,
        audio: true,
        headless: false,
        frames: 1,
        output: "output".into(),
        wav: None,
        wav_dc_blocker: false,
        budget: 4_000_000,
    };
    let mut inline = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "IBNIZ Rust {}\n\
                     Usage: ibniz [OPTIONS] [PROGRAM.ib]\n\n\
                     -c, --code CODE        Inline IBNIZ source (including empty source)\n\
                     -n, --paused           Start paused\n\
                     --no-audio             Disable desktop audio output\n\
                     --headless             Export numbered 256x256 PPM frames without a window\n\
                     --frames N             Number of 60 Hz ticks to export (default: 1)\n\
                     --output DIR           Frame directory (default: output)\n\
                     --wav FILE             Also export 61440 Hz mono signed 16-bit WAV\n\
                     --dc-blocker           Apply 20 Hz DC blocker to headless WAV (default: off)\n\
                     --max-instructions N   Instruction limit per tick (default: 4000000)\n\
                     -v, --version          Version\n\n\
                     Desktop: F1 run/pause, F2 reset, F12 help, Tab editor, Ctrl/Cmd+S save, Esc close help/exit.",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            "-v" | "--version" => {
                println!("ibniz {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-c" | "--code" => inline = Some(value(&mut args, &arg)?),
            "-n" | "--paused" => options.autorun = false,
            "--no-audio" => options.audio = false,
            "--headless" => options.headless = true,
            "--frames" => options.frames = value(&mut args, &arg)?.parse()?,
            "--output" => options.output = value(&mut args, &arg)?.into(),
            "--wav" => options.wav = Some(value(&mut args, &arg)?.into()),
            "--dc-blocker" => options.wav_dc_blocker = true,
            "--max-instructions" => options.budget = value(&mut args, &arg)?.parse()?,
            "--" => {
                for path in args.by_ref() {
                    set_path(&mut options, path)?;
                }
            }
            _ if arg.starts_with('-') => {
                return Err(input_error(format!("unknown option {arg}")).into());
            }
            _ => set_path(&mut options, arg)?,
        }
    }
    if options.frames == 0 || options.budget < 4 {
        return Err(input_error(
            "--frames must be positive and --max-instructions must be at least 4",
        )
        .into());
    }
    if options.wav_dc_blocker && (!options.headless || options.wav.is_none()) {
        return Err(input_error("--dc-blocker requires --headless and --wav FILE").into());
    }
    if inline.is_some() && options.path.is_some() {
        return Err(input_error("use either --code or a source file, not both").into());
    }
    if let Some(path) = &options.path {
        options.source = fs::read_to_string(path)?;
    } else if let Some(source) = inline {
        options.source = source;
    }
    let program = Program::compile(&options.source)?;
    if options.headless {
        return render(options, program);
    }
    #[cfg(feature = "desktop")]
    {
        desktop::launch(options, program).map_err(|e| io::Error::other(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "desktop"))]
    Err(
        input_error("desktop support is disabled; use --headless or rebuild with default features")
            .into(),
    )
}

fn value(args: &mut impl Iterator<Item = String>, option: &str) -> io::Result<String> {
    args.next()
        .ok_or_else(|| input_error(format!("{option} requires a value")))
}

fn input_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn set_path(options: &mut Options, path: String) -> io::Result<()> {
    if options.path.replace(path.into()).is_some() {
        return Err(input_error("only one source file can be opened"));
    }
    Ok(())
}

fn render(options: Options, program: Program) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&options.output)?;
    let mut audio = if let Some(path) = &options.wav {
        let samples = options
            .frames
            .checked_mul(1024)
            .ok_or_else(|| input_error("audio exceeds WAV size limit"))?;
        Some(export::WavWriter::new(
            BufWriter::new(File::create(path)?),
            samples,
        )?)
    } else {
        None
    };
    let mut vm = Vm::new(program);
    let mut dc_blocker = options
        .wav_dc_blocker
        .then(|| DcBlocker::new(ibniz::SAMPLE_RATE));
    let mut incomplete = 0;
    let mut incomplete_audio = 0;
    for index in 0..options.frames {
        let mut frame = vm.render_frame(options.budget);
        if !frame.video_complete && !vm.status().stopped {
            incomplete += 1;
        }
        if !frame.audio_complete && vm.status().audio_enabled && !vm.status().stopped {
            incomplete_audio += 1;
        }
        let path = options.output.join(format!("frame-{index:06}.ppm"));
        export::write_ppm(BufWriter::new(File::create(path)?), &frame.rgb)?;
        if let Some(audio) = &mut audio {
            if let Some(filter) = &mut dc_blocker {
                for sample in &mut frame.audio {
                    *sample = (filter.process(f32::from(*sample) / 32768.0) * 32768.0)
                        .round()
                        .clamp(f32::from(i16::MIN), f32::from(i16::MAX))
                        as i16;
                }
            }
            audio.write_samples(&frame.audio)?;
        }
    }
    if let Some(audio) = audio {
        audio.finish()?;
    }
    if incomplete > 0 {
        eprintln!(
            "warning: {incomplete} video tick(s) exhausted the instruction budget; exported the last visible page"
        );
    }
    if incomplete_audio > 0 && options.wav.is_some() {
        eprintln!(
            "warning: {incomplete_audio} audio tick(s) exhausted the instruction budget; sound may contain stale buffer samples"
        );
    }
    println!(
        "Rendered {} tick(s) to {}",
        options.frames,
        options.output.display()
    );
    Ok(())
}
