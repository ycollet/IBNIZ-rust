use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ibniz::SAMPLE_RATE;

use crate::dc_blocker::DcBlocker;

struct Shared {
    samples: VecDeque<f32>,
    error: Option<String>,
    underruns: u64,
    missing_samples: u64,
    dropped: u64,
    playing: bool,
    started: bool,
    dc_blocker: DcBlocker,
}

impl Shared {
    fn next_sample(&mut self) -> Option<f32> {
        let sample = self.samples.pop_front();
        if sample.is_none() && self.playing && self.started {
            self.missing_samples += 1;
        }
        sample
    }

    fn write_output<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        output: &mut [T],
        channels: usize,
    ) {
        let missing_before = self.missing_samples;
        for frame in output.chunks_mut(channels) {
            let sample = self.next_sample().unwrap_or(0.0);
            let sample = self.dc_blocker.process(sample).clamp(-1.0, 1.0);
            for channel in frame {
                *channel = T::from_sample(sample);
            }
        }
        if self.missing_samples > missing_before {
            self.underruns += 1;
        }
    }
}

pub struct AudioStatus {
    pub error: Option<String>,
    pub underruns: u64,
    pub gap_ms: f64,
    pub dropped: u64,
}

pub struct Audio {
    _stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    device_rate: u32,
    resample_phase: u64,
}

impl Audio {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let shared = Arc::new(Mutex::new(Shared {
            samples: VecDeque::new(),
            error: None,
            underruns: 0,
            missing_samples: 0,
            dropped: 0,
            playing: false,
            started: false,
            dc_blocker: DcBlocker::new(config.sample_rate.0),
        }));
        let stream = match format {
            cpal::SampleFormat::F32 => stream::<f32>(&device, &config, shared.clone()),
            cpal::SampleFormat::I16 => stream::<i16>(&device, &config, shared.clone()),
            cpal::SampleFormat::U16 => stream::<u16>(&device, &config, shared.clone()),
            other => return Err(format!("Unsupported output sample format: {other}")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            shared,
            device_rate: config.sample_rate.0,
            resample_phase: 0,
        })
    }

    pub fn enqueue(&mut self, input: &[i16], volume: f32) {
        if input.is_empty() {
            return;
        }
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.started = true;
        if shared.samples.is_empty() {
            shared.samples.resize(self.device_rate as usize / 20, 0.0);
        }
        let end = input.len() as u64 * u64::from(self.device_rate);
        while self.resample_phase < end {
            let index = (self.resample_phase / u64::from(self.device_rate)) as usize;
            shared
                .samples
                .push_back(f32::from(input[index]) / 32768.0 * volume);
            self.resample_phase += u64::from(SAMPLE_RATE);
        }
        self.resample_phase -= end;
        let maximum = self.device_rate as usize / 4;
        if shared.samples.len() > maximum {
            let excess = shared.samples.len() - maximum;
            shared.samples.drain(..excess);
            shared.dropped += excess as u64;
        }
    }

    pub fn clear(&mut self) {
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.samples.clear();
        shared.started = false;
        self.resample_phase = 0;
    }

    pub fn set_playing(&mut self, playing: bool) {
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.playing = playing;
        if !playing {
            shared.started = false;
        }
    }

    pub fn reset_counters(&mut self) {
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.underruns = 0;
        shared.missing_samples = 0;
        shared.dropped = 0;
    }

    pub fn set_dc_blocker(&mut self, enabled: bool) {
        self.shared
            .lock()
            .expect("audio queue poisoned")
            .dc_blocker
            .set_enabled(enabled);
    }

    pub fn status(&self) -> AudioStatus {
        let shared = self.shared.lock().expect("audio queue poisoned");
        AudioStatus {
            error: shared.error.clone(),
            underruns: shared.underruns,
            gap_ms: shared.missing_samples as f64 * 1000.0 / f64::from(self.device_rate),
            dropped: shared.dropped,
        }
    }
}

fn stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Mutex<Shared>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let errors = shared.clone();
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                let mut shared = shared.lock().expect("audio queue poisoned");
                shared.write_output(output, channels);
            },
            move |error| {
                errors.lock().expect("audio queue poisoned").error = Some(error.to_string());
            },
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> Shared {
        Shared {
            samples: VecDeque::new(),
            error: None,
            underruns: 0,
            missing_samples: 0,
            dropped: 0,
            playing: false,
            started: false,
            dc_blocker: DcBlocker::new(48_000),
        }
    }

    #[test]
    fn missing_samples_only_count_after_playback_starts() {
        let mut shared = shared();
        assert_eq!(shared.next_sample(), None);
        shared.playing = true;
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.missing_samples, 0);
        shared.started = true;
        shared.samples.push_back(0.5);
        assert_eq!(shared.next_sample(), Some(0.5));
        assert_eq!(shared.missing_samples, 0);
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.missing_samples, 2);
        shared.playing = false;
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.missing_samples, 2);
    }

    #[test]
    fn gaps_count_callbacks_and_missing_frames_not_channels() {
        let mut shared = shared();
        let mut output = [0.0_f32; 8];
        shared.write_output(&mut output, 2);
        shared.playing = true;
        shared.write_output(&mut output, 2);
        assert_eq!(shared.underruns, 0);
        shared.started = true;
        shared.samples.extend([0.0; 2]);
        shared.write_output(&mut output, 2);
        assert_eq!((shared.underruns, shared.missing_samples), (1, 2));
        shared.write_output(&mut output, 2);
        assert_eq!((shared.underruns, shared.missing_samples), (2, 6));
        shared.samples.extend([0.0; 4]);
        shared.write_output(&mut output, 2);
        assert_eq!((shared.underruns, shared.missing_samples), (2, 6));
        shared.playing = false;
        shared.write_output(&mut output, 2);
        assert_eq!((shared.underruns, shared.missing_samples), (2, 6));
    }
}
