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

pub fn write_wav(mut writer: impl Write, samples: &[i16]) -> io::Result<()> {
    let bytes = samples
        .len()
        .checked_mul(2)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n <= u32::MAX - 36)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "audio exceeds WAV size limit")
        })?;
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
    for sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }
    writer.flush()
}
