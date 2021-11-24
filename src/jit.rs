
use std::collections::HashSet;
use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};

use std::io::Write;

use super::{eval, space};

#[derive(Default)]
pub struct Block {
    pub code: String,
    pub loops: bool,
    pub mutates: bool,
}

pub struct Jit {
}

macro_rules! funjit_dynasm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            $($t)*
        )
    };
}

macro_rules! prologue {
    ($ops:ident) => {{
        let start = $ops.offset();
        funjit_dynasm!($ops
            ; sub rsp, 0x8
            ; mov [rsp], rdi
        );
        start
    }}
}

macro_rules! epilogue {
    ($ops:ident) => {
        funjit_dynasm!($ops
            ; add rsp, 0x8
            ; ret
        )
    }
}

macro_rules! call_external {
    ($ops:ident, $addr:expr) => {
        funjit_dynasm!($ops
            ; mov rdi, [rsp]
            ; mov rax, QWORD $addr as _
            ; call rax
        )
    }
}

impl Jit {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Jit {})
    }

    // Returns basic blocks from the funge space
    pub fn next_block(space: &space::Funge93, mut pc: space::Pos, mut delta: space::Pos) -> Block {
        let mut block = Block::default();
        let mut seen = HashSet::new();
        seen.insert(pc);

        loop {
            match space.get(pc) {
                '_' | '|' | '?' => break,

                'p' => block.mutates = true,

                '^' => delta = space::Pos::north(),
                '>' => delta = space::Pos::east(),
                'v' => delta = space::Pos::south(),
                '<' => delta = space::Pos::west(),

                ' ' => (),

                c => block.code.push(c),
            }

            pc += &delta;

            if seen.contains(&pc) {
                block.loops = true;
                break;
            }

            seen.insert(pc);

        }

        block
    }

    pub fn experiment(&self, eval: &mut eval::Eval) {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();

        let test = prologue!(ops);
        call_external!(ops, eval::Eval::pop);
        call_external!(ops, eval::Eval::pop);
        funjit_dynasm!(ops ; mov rsi, QWORD 42);
        call_external!(ops, eval::Eval::push);
        funjit_dynasm!(ops ; xor rax, rax);
        epilogue!(ops);

        let buf = ops.finalize().unwrap();
        let pop_fun: extern "sysv64" fn(&mut eval::Eval) -> isize = unsafe { std::mem::transmute(buf.ptr(test)) };

        println!("pop: {}\n", pop_fun(eval));
        println!("pop: {}\n", eval.pop());
        println!("pop: {}\n", eval.pop());
    }
}

extern "sysv64" fn pop(eval: &mut eval::Eval) -> isize {
    std::io::stdout().write(b"before!\n");
    eval.pop()
}
