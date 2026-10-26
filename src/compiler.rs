use std::{error::Error, fmt, ops::Range};

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
    pub(crate) source_ranges: Vec<Range<usize>>,
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
        let mut source_ranges = Vec::new();
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
                let mut start = offset;
                let mut digit_offset = offset;
                loop {
                    if digit == '.' {
                        if fractional {
                            code.push(Instruction::Immediate(value as i32));
                            source_ranges.push(start..digit_offset);
                            start = digit_offset;
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
                        Some(&(next_offset, next)) if is_number(next) => {
                            digit = next;
                            digit_offset = next_offset;
                            chars.next();
                        }
                        _ => break,
                    }
                }
                code.push(Instruction::Immediate(value as i32));
                source_ranges.push(start..digit_offset + digit.len_utf8());
            } else if "+-*/%q&|^rl~sa<>=dpxv)(zMwT@!?:;ijX[L]J{}VRPUG".contains(ch) {
                code.push(Instruction::Op(ch as u8));
                source_ranges.push(offset..offset + ch.len_utf8());
            } else {
                return Err(invalid(offset, ch));
            }
        }
        code.push(Instruction::End);
        source_ranges.push(source.len()..source.len());
        let mut hints = vec![0; code.len()];
        // The reference parser deliberately uses flat, not nested, skip points.
        let (mut conditional, mut endif, mut ret, mut media) = (0, 0, 0, 0);
        for (i, instruction) in code.iter().enumerate().rev() {
            match instruction {
                Instruction::Op(b'?') => hints[i] = conditional,
                Instruction::Op(b':') => {
                    hints[i] = endif;
                    conditional = i + 1;
                }
                Instruction::Op(b';') => {
                    endif = i + 1;
                    conditional = i + 1;
                }
                Instruction::Op(b'{') => hints[i] = ret,
                Instruction::Op(b'}') => ret = i + 1,
                Instruction::Op(b'M') => {
                    hints[i] = media;
                    media = i + 1;
                }
                _ => {}
            }
        }
        hints[code.len() - 1] = media;
        Ok(Self {
            code,
            hints,
            data,
            source_ranges,
        })
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.code
    }

    pub fn data_bits(&self) -> usize {
        self.data.len()
    }

    /// UTF-8 byte range of an instruction. The implicit End has an empty range
    /// at source EOF; comments and data do not produce instructions.
    pub fn source_range(&self, instruction: usize) -> Option<Range<usize>> {
        self.source_ranges.get(instruction).cloned()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn forward_hints(code: &[Instruction]) -> Vec<usize> {
        let mut hints = vec![0; code.len()];
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
        hints
    }

    #[test]
    fn reverse_skip_points_match_forward_reference() {
        let symbols = b"?:;{}Md1XL";
        let mut random = 1234_u64;
        for length in 0..512 {
            let source: String = (0..length)
                .map(|_| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    symbols[(random >> 32) as usize % symbols.len()] as char
                })
                .collect();
            let program = Program::compile(&source).unwrap();
            assert_eq!(program.hints, forward_hints(&program.code), "{source}");
        }
    }

    #[test]
    fn large_unterminated_conditionals_wrap_to_program_start() {
        let program = Program::compile(&"?".repeat(16384)).unwrap();
        assert_eq!(program.hints.len(), 16385);
        assert!(program.hints.iter().all(|hint| *hint == 0));
    }
}
