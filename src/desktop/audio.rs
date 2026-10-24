use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ibniz::SAMPLE_RATE;

#[derive(Default)]
struct Shared {
    samples: VecDeque<f32>,
    error: Option<String>,
    underruns: u64,
    dropped: u64,
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
        let shared = Arc::new(Mutex::new(Shared::default()));
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
        let mut shared = self.shared.lock().expect("audio queue poisoned");
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
        self.shared
            .lock()
            .expect("audio queue poisoned")
            .samples
            .clear();
        self.resample_phase = 0;
    }

    pub fn status(&self) -> (Option<String>, u64, u64) {
        let shared = self.shared.lock().expect("audio queue poisoned");
        (shared.error.clone(), shared.underruns, shared.dropped)
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
                let mut underrun = false;
                for frame in output.chunks_mut(channels) {
                    let sample = match shared.samples.pop_front() {
                        Some(sample) => sample,
                        None => {
                            underrun = true;
                            0.0
                        }
                    };
                    for channel in frame {
                        *channel = T::from_sample(sample);
                    }
                }
                if underrun {
                    shared.underruns += 1;
                }
            },
            move |error| {
                errors.lock().expect("audio queue poisoned").error = Some(error.to_string());
            },
            None,
        )
        .map_err(|e| e.to_string())
}
