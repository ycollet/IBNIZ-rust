use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ibniz"))
}

#[test]
fn help_and_version_work_without_devices() {
    let help = binary().arg("--help").output().unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("--headless"));
    assert!(help.contains("--dc-blocker"));
    assert!(help.contains("NSM_URL"));
    assert!(binary().arg("--version").status().unwrap().success());
}

#[test]
fn invalid_arguments_and_source_are_errors() {
    for args in [
        vec!["--frames", "0"],
        vec!["--max-instructions", "1"],
        vec!["--code"],
        vec!["--unknown"],
        vec!["--headless", "-c", "invalid"],
        vec!["-c", "1", "extra.ib"],
        vec!["--dc-blocker"],
        vec!["--headless", "--dc-blocker"],
        vec!["--dc-blocker", "--wav", "unused.wav"],
    ] {
        let output = binary().args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn headless_cli_exports_requested_frames_and_audio() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("ibniz-cli-{}-{unique}", std::process::id()));
    let wav = directory.join("audio.wav");
    let output = binary()
        .env(
            "NSM_URL",
            "invalid NSM URL must not affect headless exports",
        )
        .args([
            "--headless",
            "-c",
            "^xp M d3r15&*",
            "--frames",
            "3",
            "--output",
        ])
        .arg(&directory)
        .arg("--wav")
        .arg(&wav)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    for index in 0..3 {
        let bytes = fs::read(directory.join(format!("frame-{index:06}.ppm"))).unwrap();
        assert_eq!(bytes.len(), 196623);
        assert!(bytes.starts_with(b"P6\n256 256\n255\n"));
    }
    let bytes = fs::read(wav).unwrap();
    assert_eq!(bytes.len(), 44 + 3 * 1024 * 2);
    assert_eq!(&bytes[24..28], &61440_u32.to_le_bytes());
    let mut vm = ibniz::Vm::new(ibniz::Program::compile("^xp M d3r15&*").unwrap());
    let samples: Vec<_> = (0..3)
        .flat_map(|_| vm.render_frame(4_000_000).audio)
        .collect();
    let mut expected = Vec::new();
    ibniz::export::write_wav(&mut expected, &samples).unwrap();
    assert_eq!(bytes, expected);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn oversized_wav_is_rejected_before_rendering_frames() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("ibniz-wav-limit-{}-{unique}", std::process::id()));
    let output = binary()
        .args(["--headless", "-c", "", "--frames", "2097152", "--output"])
        .arg(&directory)
        .arg("--wav")
        .arg(directory.join("audio.wav"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("WAV size limit"));
    assert!(!directory.join("frame-000000.ppm").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn headless_dc_blocker_filters_wav_continuously_without_changing_video() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for (case, source) in ["^xp M p0", "^xp M d3r15&*"].into_iter().enumerate() {
        let directory = std::env::temp_dir().join(format!(
            "ibniz-dc-blocker-{}-{unique}-{case}",
            std::process::id()
        ));
        let wav = directory.join("audio.wav");
        let output = binary()
            .args(["--headless", "-c", source, "--frames", "6", "--output"])
            .arg(&directory)
            .arg("--wav")
            .arg(&wav)
            .arg("--dc-blocker")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let bytes = fs::read(wav).unwrap();
        let filtered: Vec<_> = bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&sample| i16::from_le_bytes(sample))
            .collect();
        assert_eq!(filtered.len(), 6 * 1024);
        let mut vm = ibniz::Vm::new(ibniz::Program::compile(source).unwrap());
        let mut raw = Vec::new();
        for index in 0..6 {
            let frame = vm.render_frame(4_000_000);
            let mut expected = Vec::new();
            ibniz::export::write_ppm(&mut expected, &frame.rgb).unwrap();
            assert_eq!(
                fs::read(directory.join(format!("frame-{index:06}.ppm"))).unwrap(),
                expected
            );
            raw.extend(frame.audio);
        }
        let mut unfiltered_wav = Vec::new();
        ibniz::export::write_wav(&mut unfiltered_wav, &raw).unwrap();
        assert_eq!(&bytes[..44], &unfiltered_wav[..44]);
        assert_ne!(filtered, raw);

        // Independent double-precision recurrence, spanning all tick boundaries.
        let coefficient = (-std::f64::consts::TAU * 20.0 / f64::from(ibniz::SAMPLE_RATE)).exp();
        let mut previous_input = 0.0;
        let mut previous_output = 0.0;
        for (index, (&input, &actual)) in raw.iter().zip(&filtered).enumerate() {
            let input = f64::from(input);
            let output = input - previous_input + coefficient * previous_output;
            let expected = output.round() as i16;
            assert!(
                (i32::from(actual) - i32::from(expected)).abs() <= 2,
                "case {case}, sample {index}: {actual} != {expected}"
            );
            previous_input = input;
            previous_output = output;
        }
        if case == 0 {
            assert!(raw[5 * 1024..].iter().all(|&sample| sample == i16::MIN));
            assert!(filtered[5 * 1024..].iter().all(|sample| sample.abs() <= 1));
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
