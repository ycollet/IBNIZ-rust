use std::{error::Error, fmt};

/// Parsed instruction addresses include the implicit end-of-program instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Immediate(i32),
    Op(u8),
    End,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub(crate) code: Vec<Instruction>,
    pub(crate) hints: Vec<usize>,
    pub(crate) data: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    /// Byte offset in the UTF-8 source.
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

impl Error for CompileError {}

impl Program {
    pub fn compile(source: &str) -> Result<Self, CompileError> {
        let mut code = Vec::new();
        let mut data = Vec::new();
        let mut chars = source.char_indices().peekable();
        let mut data_mode = false;
        let mut digit_size = 4;
        while let Some((offset, ch)) = chars.next() {
            if ch == '\\' {
                for (_, ch) in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }
            if ch.is_whitespace() || ch == ',' {
                continue;
            }
            if data_mode {
                match ch {
                    'b' => digit_size = 1,
                    'q' => digit_size = 2,
                    'o' => digit_size = 3,
                    'h' => digit_size = 4,
                    '0'..='9' | 'A'..='F' => {
                        let digit = ch.to_digit(16).expect("hexadecimal digit");
                        for bit in (0..digit_size).rev() {
                            data.push(digit & (1 << bit) != 0);
                        }
                    }
                    _ => return Err(invalid(offset, ch)),
                }
                continue;
            }
            if ch == '$' {
                data_mode = true;
                continue;
            }
            if is_number(ch) {
                let mut value = 0_u32;
                let mut fractional = false;
                let mut shift: u32 = 16;
                let mut digit = ch;
                loop {
                    if digit == '.' {
                        if fractional {
                            code.push(Instruction::Immediate(value as i32));
                            value = 0;
                        }
                        fractional = true;
                        shift = 12;
                    } else {
                        let n = digit.to_digit(16).expect("hexadecimal digit");
                        if !fractional {
                            value = value.rotate_left(4);
                        }
                        value |= n << shift;
                        if fractional {
                            shift = shift.wrapping_sub(4) & 31;
                        }
                    }
                    match chars.peek() {
                        Some(&(_, next)) if is_number(next) => {
                            digit = next;
                            chars.next();
                        }
                        _ => break,
                    }
                }
                code.push(Instruction::Immediate(value as i32));
            } else if "+-*/%q&|^rl~sa<>=dpxv)(zMwT@!?:;ijX[L]J{}VRPUG".contains(ch) {
                code.push(Instruction::Op(ch as u8));
            } else {
                return Err(invalid(offset, ch));
            }
        }
        code.push(Instruction::End);
        let mut hints = vec![0; code.len()];
        // The reference parser deliberately uses flat, not nested, skip points.
        for (i, instruction) in code.iter().enumerate() {
            let (start, targets): (usize, &[u8]) = match instruction {
                Instruction::End => (0, b"M"),
                Instruction::Op(b'M') => (i + 1, b"M"),
                Instruction::Op(b'?') => (i + 1, b";:"),
                Instruction::Op(b':') => (i + 1, b";"),
                Instruction::Op(b'{') => (i + 1, b"}"),
                _ => continue,
            };
            for (j, candidate) in code.iter().enumerate().skip(start) {
                match candidate {
                    Instruction::End => break,
                    Instruction::Op(op) if targets.contains(op) => {
                        hints[i] = if i == j { 0 } else { j + 1 };
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(Self { code, hints, data })
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.code
    }

    pub fn data_bits(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn read_bits(&self, pointer: &mut usize, count: u32) -> u32 {
        if self.data.is_empty() {
            return 0;
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.data[*pointer]);
            *pointer = (*pointer + 1) % self.data.len();
        }
        value
    }
}

fn is_number(ch: char) -> bool {
    matches!(ch, '0'..='9' | 'A'..='F' | '.')
}

fn invalid(offset: usize, ch: char) -> CompileError {
    CompileError {
        offset,
        message: format!("unknown IBNIZ character {ch:?}"),
    }
}
