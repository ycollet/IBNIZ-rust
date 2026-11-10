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
fn ticks_without_rgb_match_full_frames_across_slice_boundaries() {
    for source in ["^xp M d3r15&*", "", "[1]", "T", "d3.01&*"] {
        for budget in [0, 4, 4096, 1_000_000] {
            let program = Program::compile(source).unwrap();
            let mut reference = Vm::new(program.clone());
            let mut lazy = Vm::new(program);
            let mut rgb = Vec::new();
            for _ in 0..3 {
                let expected = reference.render_frame(budget);
                let mut render = lazy.begin_frame(budget);
                let tick = loop {
                    let before = lazy.status().instructions_executed;
                    let tick = render.advance_tick(&mut lazy, 257);
                    assert!(lazy.status().instructions_executed - before <= 257);
                    if let Some(tick) = tick {
                        break tick;
                    }
                };
                lazy.rgb_into(&mut rgb);
                assert_eq!(rgb, expected.rgb, "{source}, {budget}");
                assert_eq!(tick.audio, expected.audio);
                assert_eq!(tick.instructions, expected.instructions);
                assert_eq!(tick.video_complete, expected.video_complete);
                assert_eq!(tick.audio_complete, expected.audio_complete);
                assert_eq!(lazy.memory(), reference.memory());
                assert_eq!(
                    format!("{:?}", lazy.status()),
                    format!("{:?}", reference.status())
                );
            }
        }
    }
}

#[test]
fn reusable_rgb_matches_scalar_reference_and_keeps_its_allocation() {
    let mut vm = Vm::new(Program::compile("[1]").unwrap());
    let mut rgb = Vec::with_capacity(256 * 256 * 3);
    let pointer = rgb.as_ptr();
    for seed in [0_u32, 0x12345678, u32::MAX] {
        let visible = &mut vm.memory_mut()[0xf0000..0x100000];
        let mut word = seed;
        for slot in visible.iter_mut() {
            word = word.wrapping_mul(1664525).wrapping_add(1013904223);
            *slot = word as i32;
        }
        // Keep the original scalar conversion as an independent byte-exact oracle.
        let mut expected = Vec::new();
        for pair in visible.as_chunks::<2>().0 {
            let u = (pair[0] as u32 >> 16) as u8 as i8 as f64;
            let v = (pair[1] as u32 >> 24) as u8 as i8 as f64;
            for &word in pair {
                let y = (f64::from((word as u32 >> 8) as u8) - 16.0) * (255.0 / 219.0);
                for channel in [y + 1.596 * v, y - 0.392 * u - 0.813 * v, y + 2.017 * u] {
                    expected.push(channel.round().clamp(0.0, 255.0) as u8);
                }
            }
        }
        vm.rgb_into(&mut rgb);
        assert_eq!(rgb, expected);
        assert_eq!(rgb.as_ptr(), pointer);
        assert_eq!(vm.rgb(), expected);
    }
}

#[test]
fn lazy_rgb_reads_visible_memory_writes_without_a_page_flip() {
    let mut vm = Vm::new(Program::compile(".EB F0000! [1]").unwrap());
    let mut rgb = vm.rgb();
    assert_eq!(&rgb[..3], &[0, 0, 0]);
    let tick = vm
        .begin_frame(64)
        .advance_tick(&mut vm, usize::MAX)
        .unwrap();
    assert!(!tick.video_complete);
    vm.rgb_into(&mut rgb);
    assert_eq!(&rgb[..3], &[255, 255, 255]);
}

#[test]
fn profiled_frames_preserve_rendering_state_and_instruction_accounting() {
    for source in ["^xp M d3r15&*", "", "[1]", "T", "d3.01&*"] {
        for budget in [0, 1, 4, 4096, 1_000_000] {
            let program = Program::compile(source).unwrap();
            let mut reference = Vm::new(program.clone());
            let mut profiled = Vm::new(program);
            for tick in 0..3 {
                reference.set_input(tick);
                profiled.set_input(tick);
                let expected = reference.render_frame(budget);
                let (actual, profile) = profiled.render_frame_profiled(budget);
                assert_eq!(actual.rgb, expected.rgb, "{source}, {budget}");
                assert_eq!(actual.audio, expected.audio, "{source}, {budget}");
                assert_eq!(actual.instructions, expected.instructions);
                assert_eq!(actual.video_complete, expected.video_complete);
                assert_eq!(actual.audio_complete, expected.audio_complete);
                assert_eq!(
                    profile.audio_instructions + profile.video_instructions,
                    actual.instructions
                );
                assert_eq!(profiled.memory(), reference.memory());
                assert_eq!(
                    format!("{:?}", profiled.status()),
                    format!("{:?}", reference.status())
                );
            }
        }
    }
}

#[test]
fn sliced_frames_match_uninterrupted_rendering_and_instruction_limits() {
    for (source, budget) in [
        ("^xp M d3r15&*", 1_000_000),
        ("^xp", 100_000),
        ("[1]", 10_000),
        ("T", 100),
        ("^xp M d3r15&*", 0),
    ] {
        let program = Program::compile(source).unwrap();
        let mut reference = Vm::new(program.clone());
        let mut sliced = Vm::new(program);
        for _ in 0..3 {
            let expected = reference.render_frame(budget);
            let mut job = sliced.begin_frame(budget);
            assert_eq!(job.budget(), budget);
            let actual = loop {
                let before = sliced.status().instructions_executed;
                let frame = job.advance(&mut sliced, 257);
                assert!(sliced.status().instructions_executed - before <= 257);
                if let Some(frame) = frame {
                    break frame;
                }
            };
            assert_eq!(actual.rgb, expected.rgb, "{source}");
            assert_eq!(actual.audio, expected.audio, "{source}");
            assert_eq!(actual.instructions, expected.instructions, "{source}");
            assert_eq!(actual.video_complete, expected.video_complete, "{source}");
            assert_eq!(actual.audio_complete, expected.audio_complete, "{source}");
            assert_eq!(sliced.memory(), reference.memory(), "{source}");
            assert_eq!(
                sliced.status().instruction_pointer,
                reference.status().instruction_pointer
            );
            assert_eq!(sliced.status().frame, reference.status().frame);
        }
    }
}

#[test]
fn fused_stack_arithmetic_matches_single_instruction_slices() {
    for source in [
        "d3.01&*",
        "dFFFF.FFFF&* M d3.01&*",
        "d*",
        "E0004@ E0003! 0) d*",
        "M D0002@ D0001! 0) d*",
        ".0003J d* T",
        ".0003J E0004@ T",
        "2{d*} 2V T",
        "2{E0004@ 0) d*} 2V T",
        "d",
        "1",
        "d1&",
    ] {
        for budget in [0, 1, 2, 3, 4, 5, 15, 16, 17, 256] {
            let program = Program::compile(source).unwrap();
            let mut reference = Vm::new(program.clone());
            let mut fused = Vm::new(program);
            for _ in 0..3 {
                let mut job = reference.begin_frame(budget);
                let expected = loop {
                    if let Some(frame) = job.advance(&mut reference, 1) {
                        break frame;
                    }
                };
                let actual = fused.render_frame(budget);
                assert_eq!(actual.rgb, expected.rgb, "{source}, budget {budget}");
                assert_eq!(actual.audio, expected.audio, "{source}, budget {budget}");
                assert_eq!(actual.instructions, expected.instructions);
                assert_eq!(actual.video_complete, expected.video_complete);
                assert_eq!(actual.audio_complete, expected.audio_complete);
                assert_eq!(fused.memory(), reference.memory());
                assert_eq!(
                    format!("{:?}", fused.status()),
                    format!("{:?}", reference.status())
                );
            }
        }
    }
}

#[test]
fn fused_memory_preserves_aliases_and_square_preserves_wrapped_temporary_slots() {
    for (context, mask, base) in [
        (Context::Video, 0x1ffff_usize, 0xe0000),
        (Context::Audio, 0xffff_usize, 0xd0000),
    ] {
        for sp in [0, mask - 1, mask] {
            let next = base + ((sp + 1) & mask);
            let current = base + sp;
            let previous = base + (sp.wrapping_sub(1) & mask);
            for source in [
                "d*".into(),
                format!("{next:X}@"),
                format!("{next:X}!"),
                format!("{current:X}@"),
                format!("{current:X}!"),
                format!("{previous:X}!"),
                "0)".into(),
                "FFFF)".into(),
                "FFFF.FFFF)".into(),
                "7FFF.FFFF)".into(),
            ] {
                for value in [i32::MIN, i32::MAX] {
                    let prepare = || {
                        let initial = if context == Context::Audio {
                            "M [1]"
                        } else {
                            "[1]"
                        };
                        let mut vm = Vm::new(Program::compile(initial).unwrap());
                        if context == Context::Audio {
                            vm.render_frame(4);
                        }
                        assert_eq!(vm.status().context, context);
                        let initial_sp = match context {
                            Context::Video => vm.status().video_stack_pointer,
                            Context::Audio => vm.status().audio_stack_pointer,
                        };
                        for _ in 0..(sp.wrapping_sub(initial_sp) & mask) {
                            vm.push(0x12345678);
                        }
                        vm.memory_mut()[current] = value;
                        vm.replace_program(Program::compile(&source).unwrap());
                        vm
                    };
                    for budget in [1, 2, 3, 7] {
                        let mut reference = prepare();
                        let mut fused = prepare();
                        let mut job = reference.begin_frame(budget);
                        let expected = loop {
                            if let Some(frame) = job.advance(&mut reference, 1) {
                                break frame;
                            }
                        };
                        let actual = fused.render_frame(budget);
                        assert_eq!(
                            fused.memory(),
                            reference.memory(),
                            "{source}, {context:?}, sp={sp}, {value}, budget={budget}"
                        );
                        assert_eq!(actual.rgb, expected.rgb);
                        assert_eq!(actual.audio, expected.audio);
                        assert_eq!(actual.instructions, expected.instructions);
                        assert_eq!(actual.video_complete, expected.video_complete);
                        assert_eq!(actual.audio_complete, expected.audio_complete);
                        assert_eq!(
                            format!("{:?}", fused.status()),
                            format!("{:?}", reference.status())
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn live_recompilation_refreshes_fusions_during_partial_ticks() {
    for (before, after) in [
        ("^xp", "E0004@"),
        ("d*", "[1]"),
        ("d3.01&*", "d*"),
        ("E0004@", "0) d*"),
    ] {
        let program = Program::compile(before).unwrap();
        let mut reference = Vm::new(program.clone());
        let mut fused = Vm::new(program);
        let mut slow_job = reference.begin_frame(83);
        let mut fast_job = fused.begin_frame(83);
        assert!(slow_job.advance(&mut reference, 1).is_none());
        assert!(fast_job.advance(&mut fused, 1).is_none());
        reference.replace_program(Program::compile(after).unwrap());
        fused.replace_program(Program::compile(after).unwrap());
        let expected = loop {
            if let Some(frame) = slow_job.advance(&mut reference, 1) {
                break frame;
            }
        };
        let actual = loop {
            if let Some(frame) = fast_job.advance(&mut fused, 7) {
                break frame;
            }
        };
        assert_eq!(fused.memory(), reference.memory(), "{before} -> {after}");
        assert_eq!(actual.rgb, expected.rgb);
        assert_eq!(actual.audio, expected.audio);
        assert_eq!(actual.instructions, expected.instructions);
        assert_eq!(
            format!("{:?}", fused.status()),
            format!("{:?}", reference.status())
        );
    }
}

#[test]
fn bundled_examples_fused_ticks_match_single_instruction_ticks() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("ib") {
            continue;
        }
        let program = Program::compile(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let mut reference = Vm::new(program.clone());
        let mut fused = Vm::new(program);
        for input in [0, 0x8080, 0x8000ffff] {
            reference.set_input(input);
            fused.set_input(input);
            let mut job = reference.begin_frame(8192);
            let expected = loop {
                if let Some(frame) = job.advance(&mut reference, 1) {
                    break frame;
                }
            };
            let actual = fused.render_frame(8192);
            assert_eq!(actual.rgb, expected.rgb, "{}", path.display());
            assert_eq!(actual.audio, expected.audio, "{}", path.display());
            assert_eq!(actual.instructions, expected.instructions);
            assert_eq!(actual.video_complete, expected.video_complete);
            assert_eq!(actual.audio_complete, expected.audio_complete);
            assert_eq!(fused.memory(), reference.memory(), "{}", path.display());
            assert_eq!(
                format!("{:?}", fused.status()),
                format!("{:?}", reference.status())
            );
        }
    }
}

#[test]
fn fused_stack_arithmetic_preserves_wrapped_slots_and_signed_values() {
    for (source, context, mask) in [
        ("d3.01&* T", Context::Video, 0x1ffff),
        ("M dFFFF.FFFF&* T", Context::Audio, 0xffff),
    ] {
        for value in [i32::MIN, -1, 0, i32::MAX] {
            let prepare = || {
                let mut vm = Vm::new(Program::compile(source).unwrap());
                if context == Context::Audio {
                    vm.render_frame(4);
                    vm.replace_program(
                        Program::compile(source.strip_prefix("M ").unwrap()).unwrap(),
                    );
                }
                assert_eq!(vm.status().context, context);
                let sp = match context {
                    Context::Video => vm.status().video_stack_pointer,
                    Context::Audio => vm.status().audio_stack_pointer,
                };
                for _ in sp..mask {
                    vm.push(0);
                }
                let address = vm.data_stack_entry(context, 0).address;
                vm.memory_mut()[address] = value;
                vm
            };
            let mut reference = prepare();
            let mut fused = prepare();
            let mut job = reference.begin_frame(16);
            let expected = loop {
                if let Some(frame) = job.advance(&mut reference, 1) {
                    break frame;
                }
            };
            let actual = fused.render_frame(16);
            assert_eq!(actual.audio, expected.audio);
            assert_eq!(actual.rgb, expected.rgb);
            assert_eq!(actual.instructions, expected.instructions);
            assert_eq!(fused.memory(), reference.memory(), "{source}, {value}");
            assert_eq!(
                format!("{:?}", fused.status()),
                format!("{:?}", reference.status())
            );
        }
    }
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
fn instruction_source_ranges_cover_split_numbers_and_skip_comments_and_data() {
    let source = "\\ \u{e9}\n1..2,d M \\ tail\n$ b101";
    let program = Program::compile(source).unwrap();
    let spans: Vec<_> = (0..program.instructions().len())
        .map(|ip| &source[program.source_range(ip).unwrap()])
        .collect();
    assert_eq!(spans, ["1.", ".2", "d", "M", ""]);
    assert_eq!(program.source_range(4), Some(source.len()..source.len()));
    assert!(program.source_range(5).is_none());
    for source in ["", "$F", "\\ comment"] {
        let program = Program::compile(source).unwrap();
        assert_eq!(program.instructions(), &[Instruction::End]);
        assert_eq!(program.source_range(0), Some(source.len()..source.len()));
    }
    let program = Program::compile("...").unwrap();
    assert_eq!(
        (0..3)
            .map(|ip| program.source_range(ip).unwrap())
            .collect::<Vec<_>>(),
        [0..1, 1..2, 2..3]
    );
}

#[test]
fn debugger_stack_inspection_is_read_only_context_specific_and_cyclic() {
    let mut vm = Vm::new(Program::compile("1 P M 2 P").unwrap());
    assert_eq!(vm.data_stack_entry(Context::Video, 0).address, 0xe0003);
    assert_eq!(vm.data_stack_entry(Context::Video, 4).address, 0xfffff);
    assert_eq!(vm.data_stack_entry(Context::Audio, 2).address, 0xdffff);
    assert_eq!(vm.return_stack_entry(Context::Video, 1).address, 0xcffff);
    assert_eq!(vm.return_stack_entry(Context::Audio, 1).address, 0xcbfff);
    vm.memory_mut()[0xcffff] = -123;
    assert_eq!(vm.return_stack_entry(Context::Video, 1).value, -123);
    vm.step();
    assert_eq!(vm.data_stack_entry(Context::Video, 0).value, 1 << 16);
    vm.step();
    assert_eq!(vm.status().video_return_stack_pointer, 1);
    assert_eq!(vm.return_stack_entry(Context::Video, 0).value, 1);
    let _job = vm.begin_frame(1000);
    vm.step();
    assert_eq!(vm.status().context, Context::Audio);
    vm.step();
    vm.step();
    assert_eq!(vm.status().audio_return_stack_pointer, 1);
    assert_eq!(vm.return_stack_entry(Context::Audio, 0).value, 2);
    assert_eq!(vm.return_stack_entry(Context::Video, 0).value, 1);
    let before = vm.status();
    let memory = vm.memory().to_vec();
    for context in [Context::Video, Context::Audio] {
        for depth in [0, 1, 16, 0x20000, usize::MAX] {
            vm.data_stack_entry(context, depth);
            vm.return_stack_entry(context, depth);
        }
    }
    assert_eq!(
        vm.status().instructions_executed,
        before.instructions_executed
    );
    assert_eq!(vm.status().instruction_pointer, before.instruction_pointer);
    assert_eq!(vm.memory(), memory);
}

#[test]
fn current_instruction_mapping_follows_steps_jumps_live_compile_and_reset() {
    let mut vm = Vm::new(Program::compile("\\ header\n1 T").unwrap());
    assert_eq!(vm.current_instruction(), Instruction::Immediate(1 << 16));
    assert_eq!(vm.instruction_source_range(), 9..10);
    vm.step();
    assert_eq!(vm.current_instruction(), Instruction::Op(b'T'));
    assert_eq!(vm.instruction_source_range(), 11..12);
    vm.step();
    assert!(vm.status().stopped);
    assert_eq!(vm.instruction_source_range(), 11..12);
    vm.reset();
    assert_eq!(vm.instruction_source_range(), 9..10);
    vm.replace_program(Program::compile("0J").unwrap());
    vm.step();
    assert_eq!(vm.instruction_source_range(), 1..2);
    vm.step();
    assert_eq!(vm.instruction_source_range(), 0..1);
    vm.replace_program(Program::compile("\\ changed\nA").unwrap());
    assert_eq!(vm.instruction_source_range(), 10..11);
    vm.step();
    assert_eq!(vm.current_instruction(), Instruction::End);
    assert_eq!(vm.instruction_source_range(), 11..11);
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
        let mut lazy = Vm::new(program.clone());
        let mut vm = Vm::new(program);
        let mut rgb = Vec::new();
        for _ in 0..2 {
            let frame = vm.render_frame(4_000_000);
            assert!(frame.instructions <= 4_000_000, "{}", path.display());
            assert_eq!(frame.rgb.len(), 196608, "{}", path.display());
            assert_eq!(frame.audio.len(), 1024, "{}", path.display());
            assert!(!vm.status().stopped, "{}", path.display());
            let tick = lazy
                .begin_frame(4_000_000)
                .advance_tick(&mut lazy, usize::MAX)
                .unwrap();
            lazy.rgb_into(&mut rgb);
            assert_eq!(rgb, frame.rgb, "{}", path.display());
            assert_eq!(tick.audio, frame.audio, "{}", path.display());
            assert_eq!(tick.instructions, frame.instructions, "{}", path.display());
            assert_eq!(
                tick.video_complete,
                frame.video_complete,
                "{}",
                path.display()
            );
            assert_eq!(
                tick.audio_complete,
                frame.audio_complete,
                "{}",
                path.display()
            );
            assert_eq!(lazy.memory(), vm.memory(), "{}", path.display());
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

#[test]
fn multi_sample_audio_iterations_complete_across_buffer_rollovers() {
    for audio in ["d", "ddd", "dwd"] {
        let source = format!("^xp M {audio}");
        let mut vm = Vm::new(Program::compile(&source).unwrap());
        for tick in 0..130 {
            let frame = vm.render_frame(500_000);
            assert!(frame.video_complete, "{source}, video tick {tick}");
            assert!(frame.audio_complete, "{source}, audio tick {tick}");
            if audio == "d" {
                let next_time = ((tick + 1) * 1024 + 2) * 64;
                assert_eq!(
                    vm.data_stack_entry(Context::Audio, 0).value,
                    next_time,
                    "{source}, timestamp after tick {tick}",
                );
            }
        }
    }
}
