//! Dependency-free PPM image and signed 16-bit mono WAV output.
use std::io::{self, Write};

use crate::{HEIGHT, SAMPLE_RATE, WIDTH};

pub fn write_ppm(mut writer: impl Write, rgb: &[u8]) -> io::Result<()> {
    if rgb.len() != WIDTH * HEIGHT * 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected 256x256 RGB pixels",
        ));
    }
    write!(writer, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
    writer.write_all(rgb)?;
    writer.flush()
}

/// Streams a known number of samples without retaining the recording in memory.
/// Wrap file outputs in `std::io::BufWriter` to batch sample writes.
/// Call `finish` to verify the sample count and report buffered I/O errors.
#[must_use = "call finish to verify the sample count and flush the WAV output"]
pub struct WavWriter<W: Write> {
    writer: W,
    remaining: usize,
}

impl<W: Write> WavWriter<W> {
    pub fn new(mut writer: W, samples: usize) -> io::Result<Self> {
        let bytes = samples
            .checked_mul(2)
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n <= u32::MAX - 36)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "audio exceeds WAV size limit")
            })?;
        write_wav_header(&mut writer, bytes)?;
        Ok(Self {
            writer,
            remaining: samples,
        })
    }

    pub fn write_samples(&mut self, samples: &[i16]) -> io::Result<()> {
        if samples.len() > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "more samples than declared in WAV header",
            ));
        }
        for sample in samples {
            self.writer.write_all(&sample.to_le_bytes())?;
        }
        self.remaining -= samples.len();
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<W> {
        if self.remaining != 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("WAV output is missing {} sample(s)", self.remaining),
            ));
        }
        self.writer.flush()?;
        Ok(self.writer)
    }
}

pub fn write_wav(writer: impl Write, samples: &[i16]) -> io::Result<()> {
    let mut wav = WavWriter::new(writer, samples.len())?;
    wav.write_samples(samples)?;
    wav.finish()?;
    Ok(())
}

fn write_wav_header(writer: &mut impl Write, bytes: u32) -> io::Result<()> {
    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + bytes).to_le_bytes())?;
    writer.write_all(b"WAVEfmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&1_u16.to_le_bytes())?;
    writer.write_all(&SAMPLE_RATE.to_le_bytes())?;
    writer.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?;
    writer.write_all(&2_u16.to_le_bytes())?;
    writer.write_all(&16_u16.to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&bytes.to_le_bytes())?;
    Ok(())
}
