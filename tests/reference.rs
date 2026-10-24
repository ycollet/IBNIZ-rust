//! Golden snapshots from upstream C vm_slow.c at commit
//! 3ba06ae5f2cf5765c0e06874a093f767054b6359, built with clang -O0 -fwrapv.
//! Fields: source, stack top, video SP, return SP, FNV-1a hash over memory words.
use ibniz::{Program, Vm};

#[test]
fn matches_reference_c_interpreter_snapshots() {
    for line in include_str!("fixtures/reference.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let source = fields[0];
        let mut vm = Vm::new(Program::compile(source).unwrap());
        for _ in 0..102230 {
            if vm.status().stopped {
                break;
            }
            vm.step();
        }
        assert!(vm.status().stopped, "{source}");
        assert_eq!(
            vm.top() as u32,
            u32::from_str_radix(fields[1], 16).unwrap(),
            "top: {source}"
        );
        assert_eq!(
            vm.status().video_stack_pointer,
            usize::from_str_radix(fields[2], 16).unwrap(),
            "SP: {source}"
        );
        let hash = vm
            .memory()
            .iter()
            .fold(14695981039346656037_u64, |hash, word| {
                (hash ^ u64::from(*word as u32)).wrapping_mul(1099511628211)
            });
        assert_eq!(
            hash,
            u64::from_str_radix(fields[4], 16).unwrap(),
            "memory: {source}"
        );
    }
}
