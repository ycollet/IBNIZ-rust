use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ibniz::SAMPLE_RATE;

use crate::dc_blocker::DcBlocker;

pub const DEFAULT_BUFFER_MS: u32 = 80;

struct Shared {
    samples: VecDeque<f32>,
    error: Option<String>,
    underruns: u64,
    missing_samples: u64,
    dropped: u64,
    playing: bool,
    started: bool,
    device_rate: u32,
    minimum_buffer: usize,
    target_samples: usize,
    buffering: bool,
    gain: f32,
    last_output: f32,
    fade_step: f32,
    dc_blocker: DcBlocker,
}

impl Shared {
    fn new(device_rate: u32) -> Self {
        let target_samples = device_rate as usize * DEFAULT_BUFFER_MS as usize / 1000;
        Self {
            samples: VecDeque::with_capacity(
                device_rate as usize / 4 + device_rate as usize / 30 + 1,
            ),
            error: None,
            underruns: 0,
            missing_samples: 0,
            dropped: 0,
            playing: false,
            started: false,
            device_rate,
            minimum_buffer: target_samples,
            target_samples,
            buffering: true,
            gain: 0.0,
            last_output: 0.0,
            fade_step: 1.0 / (device_rate as f32 * 0.005).max(1.0),
            dc_blocker: DcBlocker::new(device_rate),
        }
    }

    fn clear(&mut self) {
        self.samples.clear();
        self.started = false;
        self.buffering = true;
        self.target_samples = self.minimum_buffer;
        self.gain = 0.0;
    }

    fn set_buffer_ms(&mut self, ms: u32) {
        self.minimum_buffer = self.device_rate as usize * ms.clamp(20, 200) as usize / 1000;
        self.target_samples = self.minimum_buffer;
    }

    fn next_sample(&mut self) -> Option<f32> {
        if self.buffering {
            if self.samples.len() < self.target_samples && (self.playing || self.samples.is_empty())
            {
                if self.playing && self.started {
                    self.missing_samples += 1;
                }
                return None;
            }
            self.buffering = false;
            self.gain = 0.0;
        }
        let sample = self.samples.pop_front();
        if sample.is_some() {
            if self.playing {
                self.started = true;
            }
        } else {
            self.buffering = true;
            if self.playing && self.started {
                self.missing_samples += 1;
                self.target_samples = (self.target_samples + self.device_rate as usize / 50)
                    .min(self.device_rate as usize / 5);
            }
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
            let sample = match self.next_sample() {
                Some(sample) => {
                    self.gain = (self.gain + self.fade_step).min(1.0);
                    self.last_output = sample * self.gain;
                    self.last_output
                }
                None => {
                    self.gain = 0.0;
                    self.last_output -= self.last_output.clamp(-self.fade_step, self.fade_step);
                    self.last_output
                }
            };
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
    pub buffered_ms: f64,
    pub target_ms: f64,
    pub buffering: bool,
}

pub struct Audio {
    _stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    device_rate: u32,
    resample_phase: u64,
    resampled: Vec<f32>,
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
        let shared = Arc::new(Mutex::new(Shared::new(config.sample_rate.0)));
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
            resampled: Vec::with_capacity(config.sample_rate.0 as usize / 60 + 1),
        })
    }

    pub fn enqueue(&mut self, input: &[i16], volume: f32) {
        if input.is_empty() {
            return;
        }
        self.resampled.clear();
        let end = input.len() as u64 * u64::from(self.device_rate);
        while self.resample_phase < end {
            let index = (self.resample_phase / u64::from(self.device_rate)) as usize;
            self.resampled
                .push(f32::from(input[index]) / 32768.0 * volume);
            self.resample_phase += u64::from(SAMPLE_RATE);
        }
        self.resample_phase -= end;
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.samples.extend(self.resampled.iter().copied());
        let maximum = self.device_rate as usize / 4;
        if shared.samples.len() > maximum {
            let excess = shared.samples.len() - maximum;
            shared.samples.drain(..excess);
            shared.dropped += excess as u64;
        }
    }

    pub fn clear(&mut self) {
        let mut shared = self.shared.lock().expect("audio queue poisoned");
        shared.clear();
        self.resample_phase = 0;
    }

    pub fn set_buffer_ms(&mut self, ms: u32) {
        self.shared
            .lock()
            .expect("audio queue poisoned")
            .set_buffer_ms(ms);
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
            buffered_ms: shared.samples.len() as f64 * 1000.0 / f64::from(self.device_rate),
            target_ms: shared.target_samples as f64 * 1000.0 / f64::from(self.device_rate),
            buffering: shared.buffering,
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
        let mut shared = Shared::new(1000);
        shared.set_buffer_ms(20);
        shared
    }

    #[test]
    fn missing_samples_only_count_after_playback_starts() {
        let mut shared = shared();
        assert_eq!(shared.next_sample(), None);
        shared.playing = true;
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.missing_samples, 0);
        shared.samples.extend([0.5; 19]);
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.samples.len(), 19);
        assert_eq!(shared.missing_samples, 0);
        shared.samples.push_back(0.5);
        assert_eq!(shared.next_sample(), Some(0.5));
        for _ in 0..19 {
            assert_eq!(shared.next_sample(), Some(0.5));
        }
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
        let mut output = [0.0_f32; 44];
        shared.write_output(&mut output, 2);
        shared.playing = true;
        shared.write_output(&mut output, 2);
        assert_eq!(shared.underruns, 0);
        shared.samples.extend([0.0; 20]);
        shared.write_output(&mut output, 2);
        assert_eq!((shared.underruns, shared.missing_samples), (1, 2));
        assert_eq!(shared.target_samples, 40);
        shared.write_output(&mut output[..8], 2);
        assert_eq!((shared.underruns, shared.missing_samples), (2, 6));
        shared.samples.extend([0.0; 39]);
        shared.write_output(&mut output[..8], 2);
        assert_eq!((shared.underruns, shared.missing_samples), (3, 10));
        assert_eq!(shared.samples.len(), 39);
        shared.samples.push_back(0.0);
        shared.write_output(&mut output[..8], 2);
        assert_eq!((shared.underruns, shared.missing_samples), (3, 10));
        assert!(!shared.buffering);
        shared.samples.clear();
        shared.playing = false;
        shared.write_output(&mut output[..8], 2);
        assert_eq!((shared.underruns, shared.missing_samples), (3, 10));
    }

    #[test]
    fn underruns_fade_out_and_recovery_fades_in_without_inserting_queued_silence() {
        let mut shared = shared();
        shared.dc_blocker.set_enabled(false);
        shared.playing = true;
        shared.samples.extend([1.0; 20]);
        let mut output = [0.0_f32; 20];
        shared.write_output(&mut output, 1);
        assert!((output[0] - 0.2).abs() < 0.0001);
        assert_eq!(output[19], 1.0);
        shared.write_output(&mut output[..5], 1);
        for (sample, expected) in output[..5].iter().zip([0.8, 0.6, 0.4, 0.2, 0.0]) {
            assert!((sample - expected).abs() < 0.0001);
        }
        assert_eq!(shared.missing_samples, 5);
        assert!(shared.samples.is_empty());
        shared.samples.extend([1.0; 40]);
        shared.write_output(&mut output[..5], 1);
        for (sample, expected) in output[..5].iter().zip([0.2, 0.4, 0.6, 0.8, 1.0]) {
            assert!((sample - expected).abs() < 0.0001);
        }
        assert_eq!(shared.missing_samples, 5);
    }

    #[test]
    fn adaptive_buffer_is_bounded_and_clear_restores_the_selected_minimum() {
        let mut shared = shared();
        shared.playing = true;
        for _ in 0..20 {
            shared.samples.resize(shared.target_samples, 0.0);
            while !shared.samples.is_empty() {
                assert_eq!(shared.next_sample(), Some(0.0));
            }
            assert_eq!(shared.next_sample(), None);
            assert!(shared.target_samples <= 200);
        }
        assert_eq!(shared.target_samples, 200);
        shared.clear();
        assert_eq!(shared.target_samples, 20);
        assert!(!shared.started);
        assert!(shared.buffering);
        shared.set_buffer_ms(100);
        assert_eq!(shared.target_samples, 100);
    }

    #[test]
    fn suspended_playback_can_drain_a_short_tail_without_counting_gaps() {
        let mut shared = shared();
        shared.samples.extend([0.5; 2]);
        assert_eq!(shared.next_sample(), Some(0.5));
        assert_eq!(shared.next_sample(), Some(0.5));
        assert_eq!(shared.next_sample(), None);
        assert_eq!(shared.missing_samples, 0);
    }
}
