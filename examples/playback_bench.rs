use std::{
    error::Error,
    hint::black_box,
    io,
    time::{Duration, Instant},
};

use ibniz::{Program, Vm};

const SUMMERLAB: &str = include_str!("summerlab-ibniz-latest.txt");
const SHORTEST: &str = include_str!("shortest_ibniz_programs.txt");
const DEADLINE: Duration = Duration::from_nanos(1_000_000_000 / 60);

struct Case {
    name: &'static str,
    source: &'static str,
}

fn cases() -> Result<Vec<Case>, io::Error> {
    let mut cases = Vec::new();
    for name in [
        "tv noise",
        "trippy & whining rotator",
        "plasmas",
        "polyrhythmic flasher slowing down",
    ] {
        let prefix = format!("- {name}: ");
        let source = SHORTEST
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .ok_or_else(|| invalid(format!("Missing shortest-program baseline: {name}")))?;
        cases.push(Case { name, source });
    }
    for name in [
        "binary jet",
        "gluon",
        "new new sleep 2",
        "muon",
        "angelic hysteron isogamete",
        "average butane cicada",
        "glucophage nut margarine",
        "nutcracker drag race",
        "elongate generosity potion",
        "aphasia corrodible bruise",
        "black magic sunbathing",
    ] {
        // Workshop entries wrap executable code in leading/trailing comment markers.
        let source = SUMMERLAB.lines().find_map(|line| {
            let (source, label) = line.trim().strip_prefix('\\')?.split_once('\\')?;
            label.trim().starts_with(name).then_some(source.trim())
        });
        cases.push(Case {
            name,
            source: source.ok_or_else(|| invalid(format!("Missing summerlab program: {name}")))?,
        });
    }
    Ok(cases)
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut frames = 300_usize;
    let mut warmup = 60_usize;
    let mut budget = 4_000_000_usize;
    let mut filter = String::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!(
                "Headless playback benchmark (use a release build).\n\
                 --frames N             Measured ticks per case (default: 300)\n\
                 --warmup N             Unmeasured ticks per case (default: 60)\n\
                 --max-instructions N   Instruction budget per tick (default: 4000000)\n\
                 --filter TEXT          Run cases whose names contain TEXT"
            );
            return Ok(());
        }
        if !matches!(
            arg.as_str(),
            "--frames" | "--warmup" | "--max-instructions" | "--filter"
        ) {
            return Err(invalid(format!("Unknown argument: {arg}")).into());
        }
        let value = args
            .next()
            .ok_or_else(|| invalid(format!("Missing value for {arg}")))?;
        match arg.as_str() {
            "--frames" => frames = value.parse()?,
            "--warmup" => warmup = value.parse()?,
            "--max-instructions" => budget = value.parse()?,
            "--filter" => filter = value,
            _ => unreachable!(),
        }
    }
    if frames == 0 || budget < 4 {
        return Err(
            invalid("--frames must be positive; --max-instructions must be at least 4").into(),
        );
    }
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| case.name.contains(&filter))
        .collect();
    if selected.is_empty() {
        return Err(invalid(format!("No benchmark cases match {filter:?}")).into());
    }
    eprintln!(
        "warmup={warmup}, frames={frames}, budget={budget}; \
         deadline=16.667 ms; audio misses count enabled audio only"
    );
    println!(
        "case,median_ms,p95_ms,worst_ms,late_ticks,audio_misses,video_misses,budget_hits,mean_instructions"
    );
    for case in selected {
        let program = Program::compile(case.source)
            .map_err(|error| invalid(format!("{}: {error}", case.name)))?;
        let mut vm = Vm::new(program);
        for _ in 0..warmup {
            black_box(vm.render_frame(budget));
        }
        let mut times = Vec::with_capacity(frames);
        let (mut late, mut audio_misses, mut video_misses, mut budget_hits) = (0, 0, 0, 0);
        let mut instructions = 0_u128;
        for _ in 0..frames {
            let start = Instant::now();
            let frame = black_box(vm.render_frame(budget));
            let elapsed = start.elapsed();
            times.push(elapsed);
            late += usize::from(elapsed > DEADLINE);
            audio_misses += usize::from(vm.status().audio_enabled && !frame.audio_complete);
            video_misses += usize::from(!frame.video_complete);
            budget_hits += usize::from(frame.instructions >= budget);
            instructions += frame.instructions as u128;
        }
        times.sort_unstable();
        let median = times[frames / 2].as_secs_f64() * 1000.0;
        let p95 = times[frames - frames / 20 - 1].as_secs_f64() * 1000.0;
        let worst = times[frames - 1].as_secs_f64() * 1000.0;
        println!(
            "{},{median:.3},{p95:.3},{worst:.3},{late},{audio_misses},{video_misses},\
             {budget_hits},{}",
            case.name,
            instructions / frames as u128,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_workshop_and_shortest_sources_compile_and_run_with_bounded_work() {
        for case in cases().unwrap() {
            assert!(!case.source.is_empty(), "{}", case.name);
            let program = Program::compile(case.source)
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            assert!(program.instructions().len() > 1, "{}", case.name);
            let mut vm = Vm::new(program);
            let frame = vm.render_frame(4096);
            assert!(frame.instructions <= 4096, "{}", case.name);
            assert_eq!(frame.audio.len(), 1024, "{}", case.name);
            assert_eq!(frame.rgb.len(), 256 * 256 * 3, "{}", case.name);
        }
    }
}
