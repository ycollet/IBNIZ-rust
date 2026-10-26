use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use ffmpeg_sidecar::command::{FfmpegCommand, ffmpeg_is_installed};
use ibniz::{
    Frame,
    export::{self, WavRecording},
};

const QUEUED_FRAMES: usize = 8;

struct CapturedFrame {
    rgb: Vec<u8>,
    audio: Vec<i16>,
}

pub struct CompletedRecording {
    pub directory: PathBuf,
    pub frames: u64,
    pub mp4: bool,
}

pub struct Recording {
    pub directory: PathBuf,
    pub frames: u64,
    sender: Option<SyncSender<CapturedFrame>>,
    worker: Option<JoinHandle<Result<CompletedRecording, String>>>,
}

impl Recording {
    pub fn start(parent: &Path, mp4: bool) -> Result<Self, String> {
        if parent.as_os_str().is_empty() {
            return Err("Choose an output directory for recordings".into());
        }
        if mp4 && !ffmpeg_is_installed() {
            return Err(
                "FFmpeg is unavailable. Install ffmpeg on PATH or uncheck Create MP4. \
                 No binaries are downloaded automatically."
                    .into(),
            );
        }
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create recording directory: {e}"))?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Cannot timestamp recording: {e}"))?
            .as_nanos();
        let directory = parent.join(format!("recording-{timestamp}-{}", std::process::id()));
        fs::create_dir(&directory)
            .map_err(|e| format!("Cannot create recording directory: {e}"))?;
        let wav = WavRecording::new(BufWriter::new(
            File::create_new(directory.join("sound.wav"))
                .map_err(|e| format!("Cannot create recording WAV: {e}"))?,
        ))
        .map_err(|e| format!("Cannot start recording WAV: {e}"))?;
        let (sender, receiver) = mpsc::sync_channel(QUEUED_FRAMES);
        let output = directory.clone();
        let worker = thread::Builder::new()
            .name("ibniz-recording".into())
            .spawn(move || record(output, wav, receiver, mp4))
            .map_err(|e| format!("Cannot start recording worker: {e}"))?;
        Ok(Self {
            directory,
            frames: 0,
            sender: Some(sender),
            worker: Some(worker),
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.sender.is_some()
    }

    pub fn capture(&mut self, frame: &Frame) -> Result<(), String> {
        let Some(sender) = &self.sender else {
            return Err("Recording is already finishing".into());
        };
        let captured = CapturedFrame {
            rgb: frame.rgb.clone(),
            audio: frame.audio.clone(),
        };
        match sender.try_send(captured) {
            Ok(()) => {
                self.frames += 1;
                Ok(())
            }
            Err(error) => {
                let message = match error {
                    TrySendError::Full(_) => {
                        "Recording stopped: disk writer could not keep up. \
                         Queued frames will be finalized; no frames are silently skipped."
                    }
                    TrySendError::Disconnected(_) => "Recording stopped: output worker failed.",
                };
                self.stop();
                Err(message.into())
            }
        }
    }

    pub fn stop(&mut self) {
        self.sender = None;
    }

    pub fn poll(&mut self) -> Option<Result<CompletedRecording, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        self.stop();
        Some(
            self.worker
                .take()
                .expect("finished recording worker")
                .join()
                .unwrap_or_else(|_| Err("Recording worker panicked".into())),
        )
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => eprintln!("ibniz: {error}"),
                Err(_) => eprintln!("ibniz: recording worker panicked"),
            }
        }
    }
}

fn record(
    directory: PathBuf,
    mut wav: WavRecording<BufWriter<File>>,
    receiver: Receiver<CapturedFrame>,
    mp4: bool,
) -> Result<CompletedRecording, String> {
    let mut frames = 0;
    let mut write_error = None;
    for frame in receiver {
        let result = (|| -> io::Result<()> {
            if frame.audio.len() != 1024 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected 1024 audio samples per recording tick",
                ));
            }
            export::write_ppm(
                BufWriter::new(File::create_new(
                    directory.join(format!("frame-{frames:06}.ppm")),
                )?),
                &frame.rgb,
            )?;
            wav.write_samples(&frame.audio)
        })();
        if let Err(error) = result {
            write_error = Some(error);
            break;
        }
        frames += 1;
    }
    let finalized = wav.finish();
    if let Some(error) = write_error {
        let finalize_error = finalized
            .err()
            .map(|e| format!("; WAV finalization also failed: {e}"))
            .unwrap_or_default();
        return Err(format!(
            "Recording write failed after {frames} tick(s) in {}: {error}{finalize_error}. \
             MP4 conversion was not attempted.",
            directory.display(),
        ));
    }
    finalized.map_err(|e| format!("Cannot finalize {}: {e}", directory.display()))?;
    if mp4 && frames > 0 {
        encode_mp4(&directory).map_err(|e| {
            format!(
                "MP4 conversion failed: {e}. PPM frames and sound.wav are retained in {}",
                directory.display(),
            )
        })?;
    }
    Ok(CompletedRecording {
        directory,
        frames,
        mp4: mp4 && frames > 0,
    })
}

fn encode_mp4(directory: &Path) -> Result<(), String> {
    if directory
        .join("recording.mp4")
        .try_exists()
        .map_err(|e| format!("Cannot check MP4 destination: {e}"))?
    {
        return Err("recording.mp4 already exists; refusing to overwrite it".into());
    }
    let mut command = FfmpegCommand::new();
    command
        .no_overwrite()
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-nostats"])
        .args(["-framerate", "60", "-start_number", "0"])
        .input("frame-%06d.ppm")
        .input("sound.wav")
        .args(["-map", "0:v:0", "-map", "1:a:0"])
        .codec_video("libx264")
        .args(["-pix_fmt", "yuv420p"])
        .codec_audio("aac")
        .args(["-ar", "48000", "-movflags", "+faststart", "-shortest"])
        .output("recording.mp4");
    let output = command
        .as_inner_mut()
        .current_dir(directory)
        .output()
        .map_err(|e| format!("Cannot run FFmpeg: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "FFmpeg exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibniz::{HEIGHT, WIDTH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "ibniz-recording-test-{timestamp}-{}",
                std::process::id(),
            )))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if self.0.exists() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }

    fn frame(value: i16) -> Frame {
        Frame {
            rgb: vec![128; WIDTH * HEIGHT * 3],
            audio: vec![value; 1024],
            instructions: 1,
            video_complete: true,
            audio_complete: true,
        }
    }

    fn finish(mut recording: Recording) -> Result<CompletedRecording, String> {
        recording.stop();
        recording.worker.take().unwrap().join().unwrap()
    }

    #[test]
    fn worker_records_all_frames_and_finalizes_exact_pcm_without_ffmpeg() {
        let parent = TestDirectory::new();
        let mut recording = Recording::start(&parent.0, false).unwrap();
        for value in [123, -456, i16::MAX] {
            recording.capture(&frame(value)).unwrap();
        }
        assert_eq!(recording.frames, 3);
        let completed = finish(recording).unwrap();
        assert_eq!(completed.frames, 3);
        assert!(!completed.mp4);
        let mut expected = Vec::new();
        export::write_wav(
            &mut expected,
            &[vec![123; 1024], vec![-456; 1024], vec![i16::MAX; 1024]].concat(),
        )
        .unwrap();
        assert_eq!(
            fs::read(completed.directory.join("sound.wav")).unwrap(),
            expected
        );
        for index in 0..3 {
            let ppm = fs::read(completed.directory.join(format!("frame-{index:06}.ppm"))).unwrap();
            assert!(ppm.starts_with(b"P6\n256 256\n255\n"));
            assert_eq!(ppm.len(), 15 + WIDTH * HEIGHT * 3);
        }
        assert!(!completed.directory.join("recording.mp4").exists());
    }

    #[test]
    fn takes_use_new_directories_and_empty_recordings_have_valid_wav_files() {
        let parent = TestDirectory::new();
        let first = finish(Recording::start(&parent.0, false).unwrap()).unwrap();
        let second = finish(Recording::start(&parent.0, false).unwrap()).unwrap();
        assert_ne!(first.directory, second.directory);
        assert_eq!(first.frames, 0);
        let mut expected = Vec::new();
        export::write_wav(&mut expected, &[]).unwrap();
        assert_eq!(
            fs::read(first.directory.join("sound.wav")).unwrap(),
            expected
        );
    }

    #[test]
    fn queue_overflow_and_worker_disconnection_stop_capture_explicitly() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut recording = Recording {
            directory: PathBuf::new(),
            frames: 0,
            sender: Some(sender),
            worker: None,
        };
        recording.capture(&frame(1)).unwrap();
        assert!(
            recording
                .capture(&frame(2))
                .unwrap_err()
                .contains("keep up")
        );
        assert_eq!(recording.frames, 1);
        assert!(!recording.is_capturing());
        assert_eq!(receiver.recv().unwrap().audio, vec![1; 1024]);
        assert!(receiver.recv().is_err());

        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        recording.sender = Some(sender);
        assert!(
            recording
                .capture(&frame(3))
                .unwrap_err()
                .contains("worker failed")
        );
        assert!(!recording.is_capturing());
    }

    #[test]
    fn write_failure_finalizes_previous_audio_and_reports_an_error() {
        let parent = TestDirectory::new();
        let mut recording = Recording::start(&parent.0, false).unwrap();
        let directory = recording.directory.clone();
        recording.capture(&frame(123)).unwrap();
        let mut invalid = frame(456);
        invalid.rgb.clear();
        recording.capture(&invalid).unwrap();
        let error = finish(recording).err().unwrap();
        assert!(error.contains("write failed after 1 tick(s)"), "{error}");
        let mut expected = Vec::new();
        export::write_wav(&mut expected, &[123; 1024]).unwrap();
        assert_eq!(fs::read(directory.join("sound.wav")).unwrap(), expected);
        assert!(!directory.join("recording.mp4").exists());
    }

    #[test]
    fn invalid_output_directories_report_errors() {
        assert!(Recording::start(Path::new(""), false).is_err());
        let parent = TestDirectory::new();
        fs::write(&parent.0, "not a directory").unwrap();
        assert!(Recording::start(&parent.0, false).is_err());
        fs::remove_file(&parent.0).unwrap();
    }

    #[test]
    fn mp4_failure_preserves_source_media() {
        let parent = TestDirectory::new();
        fs::create_dir(&parent.0).unwrap();
        fs::write(parent.0.join("recording.mp4"), b"existing video").unwrap();
        let wav = WavRecording::new(BufWriter::new(
            File::create_new(parent.0.join("sound.wav")).unwrap(),
        ))
        .unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        sender
            .send(CapturedFrame {
                rgb: frame(123).rgb,
                audio: frame(123).audio,
            })
            .unwrap();
        drop(sender);
        let error = record(parent.0.clone(), wav, receiver, true).err().unwrap();
        assert!(error.contains("PPM frames and sound.wav are retained"));
        assert!(parent.0.join("frame-000000.ppm").exists());
        let mut expected = Vec::new();
        export::write_wav(&mut expected, &[123; 1024]).unwrap();
        assert_eq!(fs::read(parent.0.join("sound.wav")).unwrap(), expected);
        assert_eq!(
            fs::read(parent.0.join("recording.mp4")).unwrap(),
            b"existing video"
        );
    }

    #[test]
    #[ignore = "requires FFmpeg with libx264/AAC and ffprobe installed"]
    fn ffmpeg_encodes_recorded_video_and_audio() {
        let parent = TestDirectory::new();
        let mut recording = Recording::start(&parent.0, true).unwrap();
        for value in [0, 123, -456] {
            recording.capture(&frame(value)).unwrap();
        }
        let completed = finish(recording).unwrap();
        assert!(completed.mp4);
        assert!(completed.directory.join("sound.wav").exists());
        assert!(completed.directory.join("frame-000002.ppm").exists());
        let output = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,width,height,r_frame_rate,nb_frames,sample_rate",
                "-of",
                "default=noprint_wrappers=1",
            ])
            .arg(completed.directory.join("recording.mp4"))
            .output()
            .unwrap();
        assert!(output.status.success());
        let streams = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "codec_name=h264",
            "width=256",
            "height=256",
            "r_frame_rate=60/1",
            "nb_frames=3",
            "codec_name=aac",
            "sample_rate=48000",
        ] {
            assert!(streams.contains(expected), "{expected}: {streams}");
        }
    }
}
