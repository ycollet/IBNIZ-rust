use crate::{Instruction, Program};

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 256;
pub const SAMPLE_RATE: u32 = 61_440;
pub const MEMORY_WORDS: usize = 0x100000;
const AUDIO_BASE: usize = 0xd0000;
const VIDEO_BASE: usize = 0xe0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Video = 0,
    Audio = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoMode {
    Tyx,
    T,
}

#[derive(Clone, Copy, Debug)]
struct Registers {
    sp: usize,
    rsp: usize,
    previous_sp: usize,
    stack_change: i16,
    where_count: i32,
    stable_where_count: i32,
}

impl Registers {
    fn new(sp: usize) -> Self {
        Self {
            sp,
            rsp: 0,
            previous_sp: 0,
            stack_change: 0,
            where_count: 0,
            stable_where_count: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VmStatus {
    pub context: Context,
    pub video_mode: VideoMode,
    pub instruction_pointer: usize,
    pub video_stack_pointer: usize,
    pub audio_stack_pointer: usize,
    pub frame: u32,
    pub stopped: bool,
    pub audio_enabled: bool,
    pub instructions_executed: u64,
}

pub struct Frame {
    /// Packed RGB, top to bottom, left to right.
    pub rgb: Vec<u8>,
    /// 1024 signed mono samples at 61440 Hz (one 60 Hz frame).
    pub audio: Vec<i16>,
    pub instructions: usize,
    /// False if the instruction budget expired before the next video page flip.
    pub video_complete: bool,
    /// False when audio was disabled or could not keep up with the budget.
    pub audio_complete: bool,
}

/// A resumable, instruction-bounded tick. Resume on the same VM without calling
/// `step`, `reset` or another frame renderer between slices. Live input and
/// program replacement are supported; discard this job when resetting the VM.
pub struct FrameRender {
    budget: usize,
    instructions: usize,
    audio_start: usize,
    audio_target: u64,
    audio: Option<Vec<i16>>,
    audio_complete: bool,
    finished: bool,
}

impl FrameRender {
    pub fn instructions(&self) -> usize {
        self.instructions
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Executes at most `slice_instructions` instructions. Returns the completed
    /// tick only after its normal completion or total instruction-budget limit.
    /// A completed job must not be resumed.
    pub fn advance(&mut self, vm: &mut Vm, slice_instructions: usize) -> Option<Frame> {
        assert!(!self.finished, "cannot resume a completed frame");
        let mut remaining = slice_instructions;
        if self.audio.is_none() {
            while vm.audio_enabled && !vm.stopped && self.instructions < self.budget / 4 {
                let writer =
                    u64::from(vm.audio_time / 64) * 65536 + vm.registers[1].previous_sp as u64;
                if writer > self.audio_target {
                    break;
                }
                if remaining == 0 {
                    return None;
                }
                vm.step();
                self.instructions += 1;
                remaining -= 1;
            }
            let writer = u64::from(vm.audio_time / 64) * 65536 + vm.registers[1].previous_sp as u64;
            self.audio_complete = vm.audio_enabled && writer > self.audio_target;
            self.audio = Some(
                (0..1024)
                    .map(|i| {
                        if vm.audio_enabled {
                            (vm.memory[AUDIO_BASE + ((self.audio_start + i) & 0xffff)] as u16
                                ^ 0x8000) as i16
                        } else {
                            0
                        }
                    })
                    .collect(),
            );
            vm.audio_read = (vm.audio_read + 1024) & 0xffff;
            vm.preferred = Context::Video;
            vm.page_flipped = false;
        }
        while !vm.stopped && !vm.page_flipped && self.instructions < self.budget {
            if remaining == 0 {
                return None;
            }
            vm.step();
            self.instructions += 1;
            remaining -= 1;
        }
        self.finished = true;
        let rgb = vm.rgb();
        vm.frame = vm.frame.wrapping_add(1);
        Some(Frame {
            rgb,
            audio: self.audio.take().expect("completed audio phase"),
            instructions: self.instructions,
            video_complete: vm.page_flipped,
            audio_complete: self.audio_complete,
        })
    }
}

/// Persistent 32-bit, 16.16 fixed-point IBNIZ machine with cyclic stacks.
pub struct Vm {
    program: Program,
    memory: Vec<i32>,
    registers: [Registers; 2],
    context: Context,
    preferred: Context,
    ip: usize,
    data_pointer: usize,
    video_mode: VideoMode,
    visible_page: usize,
    frame: u32,
    video_time: u32,
    audio_time: u32,
    audio_read: usize,
    audio_enabled: bool,
    input: u32,
    stopped: bool,
    page_flipped: bool,
    instructions_executed: u64,
}

impl Vm {
    pub fn new(program: Program) -> Self {
        let mut vm = Self {
            program,
            memory: vec![0; MEMORY_WORDS],
            registers: [Registers::new(0), Registers::new(1)],
            context: Context::Video,
            preferred: Context::Video,
            ip: 0,
            data_pointer: 0,
            video_mode: VideoMode::Tyx,
            visible_page: 1,
            frame: 0,
            video_time: 0,
            audio_time: 0,
            audio_read: 1,
            audio_enabled: true,
            input: 0,
            stopped: false,
            page_flipped: false,
            instructions_executed: 0,
        };
        if !vm.program.data.is_empty() {
            // The word pattern repeats after bit_length / gcd(bit_length, 32) words.
            let bits = vm.program.data.len();
            let period = bits >> bits.trailing_zeros().min(5);
            let mut filled = period.min(vm.memory.len());
            for word in &mut vm.memory[..filled] {
                *word = vm.program.read_bits(&mut vm.data_pointer, 32) as i32;
            }
            while filled < vm.memory.len() {
                let count = filled.min(vm.memory.len() - filled);
                vm.memory.copy_within(..count, filled);
                filled += count;
            }
            vm.data_pointer = 0;
        }
        vm.where_am_i();
        vm
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.program.clone());
    }

    /// Live recompilation preserves memory, stack pointers, timers and input.
    pub fn replace_program(&mut self, program: Program) {
        self.program = program;
        self.ip %= self.program.code.len();
        self.data_pointer = if self.program.data.is_empty() {
            0
        } else {
            self.data_pointer % self.program.data.len()
        };
        for registers in &mut self.registers {
            registers.stable_where_count = 0;
            registers.where_count = 0;
        }
        self.audio_enabled = true;
    }

    pub fn status(&self) -> VmStatus {
        VmStatus {
            context: self.context,
            video_mode: self.video_mode,
            instruction_pointer: self.ip,
            video_stack_pointer: self.registers[0].sp,
            audio_stack_pointer: self.registers[1].sp,
            frame: self.frame,
            stopped: self.stopped,
            audio_enabled: self.audio_enabled,
            instructions_executed: self.instructions_executed,
        }
    }

    pub fn memory(&self) -> &[i32] {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut [i32] {
        &mut self.memory
    }

    /// MMKK.YYXX: buttons/modifiers, last character, mouse Y, mouse X.
    pub fn set_input(&mut self, word: u32) {
        self.input = word;
    }

    pub fn input(&self) -> u32 {
        self.input
    }

    pub fn top(&self) -> i32 {
        self.memory[self.stack_base() + self.reg().sp]
    }

    pub fn push(&mut self, value: i32) {
        self.move_sp(1);
        self.set_top(value);
    }

    fn pop(&mut self) -> i32 {
        let value = self.top();
        self.move_sp(-1);
        value
    }

    fn reg(&self) -> &Registers {
        &self.registers[self.context as usize]
    }

    fn reg_mut(&mut self) -> &mut Registers {
        &mut self.registers[self.context as usize]
    }

    fn stack_base(&self) -> usize {
        if self.context == Context::Video {
            VIDEO_BASE
        } else {
            AUDIO_BASE
        }
    }

    fn stack_mask(&self) -> usize {
        if self.context == Context::Video {
            0x1ffff
        } else {
            0xffff
        }
    }

    fn return_base(&self) -> usize {
        if self.context == Context::Video {
            0xcc000
        } else {
            0xc8000
        }
    }

    fn move_sp(&mut self, delta: isize) {
        self.reg_mut().sp = self.reg().sp.wrapping_add_signed(delta) & self.stack_mask();
    }

    fn set_top(&mut self, value: i32) {
        let address = self.stack_base() + self.reg().sp;
        self.memory[address] = value;
    }

    fn rpush(&mut self, value: u32) {
        self.reg_mut().rsp = (self.reg().rsp + 1) & 0x3fff;
        let slot = self.return_base() + self.reg().rsp;
        self.memory[slot] = value as i32;
    }

    fn rtop(&self, depth: usize) -> u32 {
        self.memory[self.return_base() + (self.reg().rsp.wrapping_sub(depth) & 0x3fff)] as u32
    }

    fn rpop(&mut self) -> u32 {
        let value = self.rtop(0);
        self.reg_mut().rsp = self.reg().rsp.wrapping_sub(1) & 0x3fff;
        value
    }

    fn where_am_i(&mut self) {
        let p = self.reg().sp & 0xffff;
        self.reg_mut().where_count = self.reg().where_count.wrapping_add(1);
        match self.context {
            Context::Audio => {
                if p == 0 {
                    self.audio_time = self.audio_time.wrapping_add(64);
                }
                self.move_sp(1);
                let value = self
                    .audio_time
                    .wrapping_mul(65536)
                    .wrapping_add((self.reg().sp as u32).wrapping_mul(64));
                self.set_top(value as i32);
            }
            Context::Video => {
                let flip = match self.video_mode {
                    VideoMode::Tyx => self.visible_page == self.reg().sp >> 16,
                    VideoMode::T => p == 0,
                };
                if flip {
                    self.visible_page = (self.reg().sp >> 16) ^ 1;
                    self.video_time = self.frame;
                    self.page_flipped = true;
                }
                match self.video_mode {
                    VideoMode::Tyx => {
                        self.push((self.video_time << 16) as i32);
                        self.push(((p as i32) << 1) - 0x10000);
                        self.push((((p & 255) as i32) << 9) - 0x10000);
                    }
                    VideoMode::T => self.push(((self.video_time << 16) | p as u32) as i32),
                }
            }
        }
    }

    fn media_boundary(&mut self, hint: usize, end: bool) {
        let preferred = self.preferred;
        let current = self.context;
        let r = self.reg_mut();
        let change = r.sp.wrapping_sub(r.previous_sp) as i16;
        if i32::from(change).abs() < 1024 {
            r.stable_where_count = if r.stack_change == change {
                r.where_count
            } else {
                0
            };
            r.stack_change = change;
        }
        r.where_count = 0;
        r.previous_sp = r.sp;
        if current == preferred {
            self.ip = hint;
        } else {
            self.context = preferred;
            if end {
                self.ip = 0;
            }
        }
        self.detect_media_formats();
        self.where_am_i();
    }

    fn detect_media_formats(&mut self) {
        let audio = self.registers[1];
        if audio.stable_where_count > 0 && audio.stack_change <= 0 {
            self.audio_enabled = false;
        }
        let video = self.registers[0];
        if video.stable_where_count > 0 {
            let delta = i64::from(video.stack_change);
            let n = i64::from(video.stable_where_count);
            if self.video_mode == VideoMode::Tyx && delta - n * 2 == 1 {
                self.video_mode = VideoMode::T;
            } else if self.video_mode == VideoMode::T && delta + n * 2 == 1 {
                self.video_mode = VideoMode::Tyx;
            }
        }
    }

    /// Execute one instruction. All overflow and address wrapping is explicit.
    pub fn step(&mut self) {
        if self.stopped {
            return;
        }
        let index = self.ip;
        let instruction = self.program.code[index];
        self.ip = (self.ip + 1) % self.program.code.len();
        self.instructions_executed = self.instructions_executed.wrapping_add(1);
        let a = self.top();
        match instruction {
            Instruction::Immediate(value) => self.push(value),
            Instruction::End => self.media_boundary(self.program.hints[index], true),
            Instruction::Op(op) => match op {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'r' | b'l' | b'a' => {
                    self.pop();
                    let b = self.top();
                    let value = match op {
                        b'+' => b.wrapping_add(a),
                        b'-' => b.wrapping_sub(a),
                        b'*' => ((i64::from(b) * i64::from(a)) >> 16) as i32,
                        b'/' => {
                            if a == 0 {
                                0
                            } else {
                                ((i64::from(b) << 16) / i64::from(a)) as i32
                            }
                        }
                        b'%' => b.checked_rem(a).unwrap_or(0),
                        b'&' => b & a,
                        b'|' => b | a,
                        b'^' => b ^ a,
                        b'r' => (b as u32).rotate_right(((a >> 16) & 31) as u32) as i32,
                        // Match the actual IBNIZ_SHL macro, including its signed-right branch.
                        b'l' => {
                            if a & (32 << 16) != 0 {
                                b >> (((!a) >> 16) & 31)
                            } else {
                                b.wrapping_shl(((a >> 16) & 31) as u32)
                            }
                        }
                        b'a' => {
                            (f64::from(b).atan2(f64::from(a)) * (65536.0 / std::f64::consts::TAU))
                                as i32
                        }
                        _ => unreachable!(),
                    };
                    self.set_top(value);
                }
                b'q' => self.set_top(if a < 0 {
                    0
                } else {
                    (f64::from(a).sqrt() * 256.0) as i32
                }),
                b's' => self.set_top(
                    (f64::from(a)
                        .mul_add(std::f64::consts::TAU / 65536.0, 0.0)
                        .sin()
                        * 65536.0) as i32,
                ),
                b'~' => self.set_top(!a),
                b'<' => self.set_top(a.min(0)),
                b'>' => self.set_top(a.max(0)),
                b'=' => self.set_top(if a == 0 { 65536 } else { 0 }),
                b'd' => self.push(a),
                b'p' => {
                    self.pop();
                }
                b'x' => {
                    let b = self.stack_base() + (self.reg().sp.wrapping_sub(1) & self.stack_mask());
                    let top = self.stack_base() + self.reg().sp;
                    self.memory.swap(top, b);
                }
                b'v' => {
                    let base = self.stack_base();
                    let sp = self.reg().sp;
                    let b = base + (sp.wrapping_sub(1) & self.stack_mask());
                    let c = base + (sp.wrapping_sub(2) & self.stack_mask());
                    let value = self.memory[c];
                    self.memory[c] = self.memory[b];
                    self.memory[b] = a;
                    self.set_top(value);
                }
                b')' => {
                    let address = self
                        .reg()
                        .sp
                        .wrapping_sub(1)
                        .wrapping_sub((a as u32).rotate_left(16) as usize)
                        & self.stack_mask();
                    self.set_top(self.memory[self.stack_base() + address]);
                }
                b'(' => {
                    self.pop();
                    let value = self.pop();
                    let address = self
                        .reg()
                        .sp
                        .wrapping_sub((a as u32).rotate_left(16) as usize)
                        & self.stack_mask();
                    let slot = self.stack_base() + address;
                    self.memory[slot] = value;
                }
                b'z' => {
                    self.move_sp(1);
                    self.set_top(
                        ((self.stack_base() + self.reg().sp) as u32).rotate_left(16) as i32
                    );
                }
                b'M' => self.media_boundary(self.program.hints[index], false),
                b'w' => self.where_am_i(),
                b'T' => {
                    self.ip = index;
                    self.stopped = true;
                }
                b'@' => self.set_top(self.memory[address(a)]),
                b'!' => {
                    self.pop();
                    let value = self.pop();
                    self.memory[address(a)] = value;
                }
                b'?' => {
                    self.pop();
                    if a == 0 {
                        self.ip = self.program.hints[index];
                    }
                }
                b':' => self.ip = self.program.hints[index],
                b';' => {}
                b'i' => self.push(self.rtop(1).rotate_left(16) as i32),
                b'j' => self.push(self.rtop(3).rotate_left(16) as i32),
                b'X' => {
                    self.pop();
                    self.rpush((a as u32).rotate_left(16));
                    self.rpush(self.ip as u32);
                }
                b'[' => self.rpush(self.ip as u32),
                b'L' => {
                    let count = self.rtop(1).wrapping_sub(1);
                    let slot = self.return_base() + (self.reg().rsp.wrapping_sub(1) & 0x3fff);
                    self.memory[slot] = count as i32;
                    if count == 0 {
                        self.rpop();
                        self.rpop();
                    } else {
                        self.ip = self.rtop(0) as usize % self.program.code.len();
                    }
                }
                b']' => {
                    self.pop();
                    if a != 0 {
                        self.ip = self.rtop(0) as usize % self.program.code.len();
                    } else {
                        self.rpop();
                    }
                }
                b'J' => {
                    self.pop();
                    self.ip = (a as u32) as usize % self.program.code.len();
                }
                b'{' => {
                    self.pop();
                    self.memory[address(a)] = self.ip as i32;
                    self.ip = self.program.hints[index];
                }
                b'}' => self.ip = self.rpop() as usize % self.program.code.len(),
                b'V' => {
                    self.pop();
                    self.rpush(self.ip as u32);
                    self.ip = self.memory[address(a)] as u32 as usize % self.program.code.len();
                }
                b'R' => {
                    let value = self.rpop().rotate_left(16);
                    self.push(value as i32);
                }
                b'P' => {
                    self.pop();
                    self.rpush((a as u32).rotate_left(16));
                }
                b'U' => {
                    self.push(self.input as i32);
                    self.input &= 0xff00ffff;
                }
                b'G' => {
                    let value = self
                        .program
                        .read_bits(&mut self.data_pointer, ((a >> 16) & 31) as u32);
                    self.set_top(value.rotate_left(16) as i32);
                }
                _ => unreachable!("compiler only emits supported operations"),
            },
        }
    }

    /// Render one deterministic 1/60-second tick with a bounded instruction count.
    /// A non-producing or infinite program cannot block its caller indefinitely.
    pub fn render_frame(&mut self, max_instructions: usize) -> Frame {
        self.begin_frame(max_instructions)
            .advance(self, usize::MAX)
            .expect("an unlimited slice completes an instruction-bounded tick")
    }

    /// Begins a tick that can be resumed in small instruction slices.
    pub fn begin_frame(&mut self, max_instructions: usize) -> FrameRender {
        self.preferred = Context::Audio;
        FrameRender {
            budget: max_instructions,
            instructions: 0,
            audio_start: self.audio_read,
            audio_target: self.frame.wrapping_add(1) as u64 * 1024,
            audio: None,
            audio_complete: false,
            finished: false,
        }
    }

    /// Convert the visible page using the reference SDL YUY2 pixel pairing.
    pub fn rgb(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(WIDTH * HEIGHT * 3);
        let base = VIDEO_BASE + self.visible_page * 65536;
        for pair in self.memory[base..base + 65536].as_chunks::<2>().0 {
            let first = pair[0] as u32;
            let second = pair[1] as u32;
            let u = (first >> 16) as u8 as i8 as f64;
            let v = (second >> 24) as u8 as i8 as f64;
            for word in [first, second] {
                let y = f64::from((word >> 8) as u8);
                // Limited-range BT.601, as used by SDL's YUV overlay.
                let y = (y - 16.0) * (255.0 / 219.0);
                for channel in [y + 1.596 * v, y - 0.392 * u - 0.813 * v, y + 2.017 * u] {
                    rgb.push(channel.round().clamp(0.0, 255.0) as u8);
                }
            }
        }
        rgb
    }
}

fn address(value: i32) -> usize {
    (value as u32).rotate_left(16) as usize & (MEMORY_WORDS - 1)
}
