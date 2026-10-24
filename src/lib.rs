//! IBNIZ compiler, audiovisual virtual machine, and portable media exporters.
//!
//! This is an altered Rust implementation of Ville-Matias Heikkila's IBNIZ,
//! based on the upstream 1.18 C interpreter.

mod compiler;
pub mod export;
mod vm;

pub use compiler::{CompileError, Instruction, Program};
pub use vm::{Context, Frame, HEIGHT, MEMORY_WORDS, SAMPLE_RATE, VideoMode, Vm, VmStatus, WIDTH};
