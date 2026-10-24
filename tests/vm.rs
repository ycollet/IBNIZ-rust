use ibniz::{Context, Instruction, Program, VideoMode, Vm, export};

fn run(source: &str) -> Vm {
    let mut vm = Vm::new(Program::compile(source).unwrap());
    for _ in 0..10000 {
        if vm.status().stopped {
            return vm;
        }
        vm.step();
    }
    panic!("test program did not terminate: {source}");
}

#[test]
fn numbers_comments_and_data() {
    let p = Program::compile("ABCD.1234,1 2\n\\ comment without trailing newline").unwrap();
    assert_eq!(
        p.instructions(),
        &[
            Instruction::Immediate(0xabcd1234_u32 as i32),
            Instruction::Immediate(65536),
            Instruction::Immediate(131072),
            Instruction::End,
        ]
    );
    assert_eq!(
        Program::compile("1.2.3").unwrap().instructions(),
        &[
            Instruction::Immediate(0x12000),
            Instruction::Immediate(0x3000),
            Instruction::End,
        ]
    );
    assert_eq!(Program::compile("$b101q23o7hF").unwrap().data_bits(), 14);
    assert!(Program::compile("hello").is_err());
    assert!(Program::compile("$z").is_err());
}

#[test]
fn whitespace_separates_numbers() {
    assert_eq!(run("1 2+T").top(), 3 << 16);
    assert_eq!(run("12T").top(), 0x12 << 16);
}

#[test]
fn arithmetic_wraps_and_uses_fixed_point() {
    for (source, expected) in [
        ("1.8 2*T", 3 << 16),
        ("3 2/T", 0x18000),
        ("3.8 2%T", 0x18000),
        ("1 0/T", 0),
        ("1 0%T", 0),
        ("7FFF.FFFF .0001+T", i32::MIN),
        ("8000 8000/T", 65536),
        ("4qT", 2 << 16),
        ("FFFFqT", 0),
        (".4sT", 65536),
        ("0=T", 65536),
        ("1=T", 0),
        ("FFFF<T", -65536),
        ("FFFF>T", 0),
        ("1<T", 0),
        ("1>T", 65536),
    ] {
        assert_eq!(run(source).top(), expected, "{source}");
    }
    assert_eq!(run("8000 FFFF.FFFF%T").top(), 0);
}

#[test]
fn bitwise_and_shift_reference_semantics() {
    assert_eq!(run("AB CD&T").top(), 0x89 << 16);
    assert_eq!(run("AB CD|T").top(), 0xef << 16);
    assert_eq!(run("AB CD^T").top(), 0x66 << 16);
    assert_eq!(run("1234.5678 0rT").top(), 0x12345678);
    assert_eq!(run("1234.5678 4rT").top(), 0x81234567_u32 as i32);
    assert_eq!(run("8000 20lT").top(), -1);
    assert_eq!(run("1 1lT").top(), 2 << 16);
    assert_eq!(run("0~T").top(), -1);
    assert_eq!(run("1 0aT").top(), 0x4000);
}

#[test]
fn stack_operations() {
    assert_eq!(run("1d+T").top(), 2 << 16);
    assert_eq!(run("1 2xT").top(), 1 << 16);
    assert_eq!(run("1 2 3vT").top(), 1 << 16);
    assert_eq!(run("1 2 1)T").top(), 1 << 16);
    assert_eq!(run("1 2 3 0(T").top(), 3 << 16);
    assert_eq!(run("ppppT").status().video_stack_pointer, 0x1ffff);
    let vm = run("zT");
    assert_eq!((vm.top() as u32).rotate_left(16), 0xe0004);
}

#[test]
fn memory_addressing_wraps() {
    assert_eq!(
        run("ABCD 1234.FFFF!1234.FFFF@T").top(),
        0xabcd0000_u32 as i32
    );
    assert_eq!(run("1234.5678 0.0010!0@T").top(), 0x12345678);
}

#[test]
fn conditionals_use_flat_reference_skip_points() {
    assert_eq!(run("0?1:2;T").top(), 2 << 16);
    assert_eq!(run("1?2:3;T").top(), 2 << 16);
    assert_eq!(run("0?1?2:3;4;T").top(), 4 << 16);
}

#[test]
fn loops_subroutines_and_return_stack() {
    assert_eq!(run("0 3Xi+LT").top(), 6 << 16);
    assert_eq!(run("0 2X3Xj+LLT").top(), 9 << 16);
    assert_eq!(run("3[1-d]T").top(), 0);
    assert_eq!(run("A{2*}3 AVT").top(), 6 << 16);
    assert_eq!(run("1234.5678PRT").top(), 0x12345678);
    let mut vm = run(".0002JT");
    assert!(vm.status().stopped);
    vm.reset();
    assert!(!vm.status().stopped);
}

#[test]
fn data_is_cyclic_and_initializes_memory() {
    let mut vm = Vm::new(Program::compile("3G 3G T$b101").unwrap());
    assert_eq!(vm.memory()[0] as u32, 0xb6db6db6);
    for _ in 0..5 {
        vm.step();
    }
    assert_eq!(vm.top(), 5 << 16);
    let vm = run("1FGT$hFFFFFFFF");
    assert_eq!(vm.top() as u32, 0xffff7fff);
    assert_eq!(run("1FGT").top(), 0);
}

#[test]
fn bulk_data_initialization_matches_bitwise_cyclic_reference() {
    let mut random = 1234_u64;
    for length in [
        1, 2, 3, 5, 7, 16, 31, 32, 33, 63, 64, 65, 127, 256, 257, 4097,
    ] {
        let bits: Vec<_> = (0..length)
            .map(|_| {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((random >> 32) & 1) as u32
            })
            .collect();
        let data: String = bits
            .iter()
            .map(|bit| if *bit == 0 { '0' } else { '1' })
            .collect();
        let expected: Vec<i32> = (0..length)
            .map(|word| {
                (0..32).fold(0_u32, |value, bit| {
                    (value << 1) | bits[(word * 32 + bit) % length]
                }) as i32
            })
            .collect();
        let mut vm = Vm::new(Program::compile(&format!("1GT$b{data}")).unwrap());
        for (index, value) in vm.memory().iter().enumerate() {
            let expected = match index {
                0xe0001 => 0,
                0xe0002 | 0xe0003 => -65536,
                _ => expected[index % length],
            };
            assert_eq!(
                *value, expected,
                "data length {length}, memory word {index}"
            );
        }
        for _ in 0..2 {
            vm.step();
        }
        assert_eq!(
            vm.top(),
            (bits[0] << 16) as i32,
            "data pointer must start at zero"
        );
        vm.memory_mut()[0] = !expected[0];
        vm.reset();
        assert_eq!(vm.memory()[0], expected[0]);
        for _ in 0..2 {
            vm.step();
        }
        assert_eq!(vm.top(), (bits[0] << 16) as i32, "reset must rewind data");
    }
}

#[test]
fn data_word_period_larger_than_memory_is_truncated_without_changing_alignment() {
    let bit_length = ibniz::MEMORY_WORDS + 1;
    let source = format!("1GT$b1{}", "0".repeat(bit_length - 1));
    let mut vm = Vm::new(Program::compile(&source).unwrap());
    for word in [0, 32767, 32768, 65536, 98304, ibniz::MEMORY_WORDS - 1] {
        let expected = (0..32).fold(0_u32, |value, bit| {
            (value << 1) | u32::from((word * 32 + bit) % bit_length == 0)
        });
        assert_eq!(vm.memory()[word] as u32, expected, "word {word}");
    }
    vm.step();
    vm.step();
    assert_eq!(
        vm.top(),
        65536,
        "initialization must rewind the data pointer"
    );
}

#[test]
fn input_clears_only_character() {
    let mut vm = Vm::new(Program::compile("UUT").unwrap());
    vm.set_input(0xa3411234);
    vm.step();
    assert_eq!(vm.top() as u32, 0xa3411234);
    vm.step();
    assert_eq!(vm.top() as u32, 0xa3001234);
}

#[test]
fn live_compile_preserves_memory_and_resets_only_when_requested() {
    let mut vm = run("AB 123!T");
    vm.replace_program(Program::compile("1T").unwrap());
    assert_eq!(vm.memory()[0x123], 0xab0000);
    assert!(vm.status().stopped);
    vm.reset();
    assert_eq!(vm.memory()[0x123], 0);
}

#[test]
fn media_switch_separates_video_and_audio() {
    let mut vm = Vm::new(Program::compile("^xp M d3r15&*").unwrap());
    let frame = vm.render_frame(4_000_000);
    assert!(frame.video_complete);
    assert!(frame.audio_complete);
    assert_eq!(frame.rgb.len(), 256 * 256 * 3);
    assert_eq!(frame.audio.len(), 1024);
    assert_eq!(vm.status().video_mode, VideoMode::Tyx);
    assert_eq!(vm.status().context, Context::Video);
    for i in 1..1024 {
        let t = (i as u32 + 1) * 64;
        let expected = ((u64::from(t) * u64::from(t.rotate_right(3) & (0x15 << 16))) >> 16) as u16;
        assert_eq!(frame.audio[i], (expected ^ 0x8000) as i16, "sample {i}");
    }
}

#[test]
fn auto_video_mode_and_non_producing_audio() {
    let mut empty = Vm::new(Program::compile("").unwrap());
    assert!(empty.render_frame(1_000_000).video_complete);
    assert_eq!(empty.status().video_mode, VideoMode::T);
    let mut vm = Vm::new(Program::compile("^xp").unwrap());
    let frame = vm.render_frame(1_000_000);
    assert!(frame.video_complete);
    assert!(!vm.status().audio_enabled);
    assert!(frame.audio.iter().all(|s| *s == 0));
}

#[test]
fn infinite_program_is_bounded_and_termination_is_honored() {
    let mut vm = Vm::new(Program::compile("[1]").unwrap());
    let frame = vm.render_frame(1000);
    assert_eq!(frame.instructions, 1000);
    assert!(!frame.video_complete);
    let mut vm = Vm::new(Program::compile("T").unwrap());
    assert_eq!(vm.render_frame(1000).instructions, 1);
    assert!(vm.status().stopped);
}

#[test]
fn export_shapes_and_headers() {
    let mut ppm = Vec::new();
    export::write_ppm(&mut ppm, &vec![0; 256 * 256 * 3]).unwrap();
    assert!(ppm.starts_with(b"P6\n256 256\n255\n"));
    assert!(export::write_ppm(Vec::new(), &[]).is_err());
    let mut wav = Vec::new();
    export::write_wav(&mut wav, &[-32768, 0, 32767]).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[24..28], &61440_u32.to_le_bytes());
    assert_eq!(&wav[40..44], &6_u32.to_le_bytes());
    assert_eq!(&wav[44..], &[0, 128, 0, 0, 255, 127]);
}

#[test]
fn exporters_report_buffer_flush_failures() {
    struct FailOnFlush;
    impl std::io::Write for FailOnFlush {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            Ok(buffer.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("flush failed"))
        }
    }
    assert!(export::write_ppm(FailOnFlush, &vec![0; 256 * 256 * 3]).is_err());
    assert!(export::write_wav(FailOnFlush, &[1, 2]).is_err());
}

#[test]
fn bundled_demos_compile_and_execute_with_bounded_work() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut examples: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ib"))
        .collect();
    examples.sort();
    assert!(!examples.is_empty());
    for path in examples {
        let source = std::fs::read_to_string(&path).unwrap();
        let program =
            Program::compile(&source).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let mut vm = Vm::new(program);
        for _ in 0..2 {
            let frame = vm.render_frame(4_000_000);
            assert!(frame.instructions <= 4_000_000, "{}", path.display());
            assert_eq!(frame.rgb.len(), 196608, "{}", path.display());
            assert_eq!(frame.audio.len(), 1024, "{}", path.display());
            assert!(!vm.status().stopped, "{}", path.display());
        }
    }
}

#[test]
fn reference_xor_pixels_and_audio_buffer_rollover() {
    let mut vm = Vm::new(Program::compile("^xp M d3r15&*").unwrap());
    for tick in 0..130 {
        let frame = vm.render_frame(500_000);
        assert!(frame.video_complete, "video tick {tick}");
        assert!(frame.audio_complete, "audio tick {tick}");
        let visible = if vm.status().video_stack_pointer >> 16 == 0 {
            0xf0000
        } else {
            0xe0000
        };
        // Pixel values are written one slot after the SP used to calculate X and Y.
        for p in [0_usize, 1, 255, 256, 32768, 65534] {
            let x = (((p & 255) as i32) << 9) - 65536;
            let y = ((p as i32) << 1) - 65536;
            assert_eq!(
                vm.memory()[visible + p + 1],
                x ^ y,
                "tick {tick}, pixel {p}"
            );
        }
        for i in 2..1024 {
            let t = ((tick * 1024 + i + 1) * 64) as u32;
            let expected = ((i64::from(t as i32)
                * i64::from((t.rotate_right(3) & (0x15 << 16)) as i32))
                >> 16) as u16;
            assert_eq!(
                frame.audio[i],
                (expected ^ 0x8000) as i16,
                "tick {tick}, sample {i}"
            );
        }
    }
}
