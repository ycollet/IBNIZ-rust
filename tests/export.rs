use std::{
    cell::Cell,
    io::{self, Write},
    rc::Rc,
};

use ibniz::export::{WavWriter, write_wav};

#[test]
fn streamed_wav_matches_single_slice_export() {
    let samples: Vec<_> = (0..3001).map(|i| (i * 127) as i16).collect();
    let mut expected = Vec::new();
    write_wav(&mut expected, &samples).unwrap();
    let mut wav = WavWriter::new(Vec::new(), samples.len()).unwrap();
    wav.write_samples(&[]).unwrap();
    for chunk in samples.chunks(257) {
        wav.write_samples(chunk).unwrap();
    }
    assert_eq!(wav.finish().unwrap(), expected);
}

#[test]
fn empty_stream_has_a_valid_header() {
    let output = WavWriter::new(Vec::new(), 0).unwrap().finish().unwrap();
    assert_eq!(output.len(), 44);
    assert_eq!(&output[..4], b"RIFF");
    assert_eq!(&output[4..8], &36_u32.to_le_bytes());
    assert_eq!(&output[40..44], &0_u32.to_le_bytes());
}

#[test]
fn stream_checks_the_declared_sample_count() {
    let mut short = WavWriter::new(Vec::new(), 2).unwrap();
    short.write_samples(&[1]).unwrap();
    assert_eq!(
        short.finish().unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );

    let mut long = WavWriter::new(Vec::new(), 1).unwrap();
    assert_eq!(
        long.write_samples(&[1, 2]).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    long.write_samples(&[3]).unwrap();
    let output = long.finish().unwrap();
    assert_eq!(&output[44..], &3_i16.to_le_bytes());
}

#[test]
fn wav_rejects_oversized_lengths_before_writing() {
    let mut output = Vec::new();
    for count in [usize::MAX, (u32::MAX as usize - 36) / 2 + 1] {
        let error = WavWriter::new(&mut output, count).err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(output.is_empty());
    }
}

struct CountingWriter(Rc<Cell<usize>>);

impl Write for CountingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.set(self.0.get() + buffer.len());
        Ok(buffer.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn samples_are_written_incrementally_without_seeking_or_retaining_the_recording() {
    let bytes = Rc::new(Cell::new(0));
    let mut wav = WavWriter::new(CountingWriter(bytes.clone()), 1024 * 100).unwrap();
    assert_eq!(bytes.get(), 44);
    for frame in 1..=100 {
        wav.write_samples(&[123; 1024]).unwrap();
        assert_eq!(bytes.get(), 44 + frame * 2048);
    }
    wav.finish().unwrap();
}

#[test]
fn streaming_reports_header_sample_and_flush_errors() {
    struct FailingWriter {
        remaining: usize,
        fail_flush: bool,
    }
    impl Write for FailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("write failed"));
            }
            let written = self.remaining.min(buffer.len());
            self.remaining -= written;
            Ok(written)
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::other("flush failed"))
            } else {
                Ok(())
            }
        }
    }
    assert!(
        WavWriter::new(
            FailingWriter {
                remaining: 0,
                fail_flush: false
            },
            1
        )
        .is_err()
    );
    let mut wav = WavWriter::new(
        FailingWriter {
            remaining: 44,
            fail_flush: false,
        },
        1,
    )
    .unwrap();
    assert!(wav.write_samples(&[1]).is_err());
    let wav = WavWriter::new(
        FailingWriter {
            remaining: 44,
            fail_flush: true,
        },
        0,
    )
    .unwrap();
    assert!(wav.finish().is_err());
}
