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
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--headless")
    );
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
    fs::remove_dir_all(directory).unwrap();
}
