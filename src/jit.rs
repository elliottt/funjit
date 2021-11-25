use dynasmrt::{dynasm, mmap::ExecutableBuffer, DynasmApi, DynasmLabelApi};
use std::collections::{HashMap, HashSet};

use super::{eval, space};

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
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, 16
            ; mov [rsp], rdi
            ; ->entry:
        );
        start
    }}
}

macro_rules! set_pc {
    ($ops:ident, $pc:expr) => {
        funjit_dynasm!($ops ; mov rsi, QWORD $pc.x as _);
        funjit_dynasm!($ops ; mov rdx, QWORD $pc.y as _);
        call_external!($ops, eval::Eval::set_pc);
    }
}

macro_rules! set_delta {
    ($ops:ident, $pc:expr) => {
        funjit_dynasm!($ops ; mov rsi, QWORD $pc.x as _);
        funjit_dynasm!($ops ; mov rdx, QWORD $pc.y as _);
        call_external!($ops, eval::Eval::set_delta);
    }
}

macro_rules! epilogue {
    ($ops:ident, $terminates:expr) => {
        funjit_dynasm!($ops
            ; mov rax, QWORD $terminates as _
            ; leave
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

macro_rules! funjit_debugger {
    ($ops:ident) => {
        funjit_dynasm!($ops ; int 3);
    }
}

// pops two values from the stack, and puts them in rsi and rax respectively
macro_rules! binop {
    ($ops:ident) => {
        call_external!($ops, eval::Eval::pop);
        funjit_dynasm!($ops ; mov [rsp + 8], rax);
        call_external!($ops, eval::Eval::pop);
        funjit_dynasm!($ops ; mov rsi, [rsp + 8]);
    }
}

#[derive(Default)]
pub struct Block {
    pub code: String,
    pub loops: bool,
    pub mutates: bool,
    pub terminates: bool,
    pub pc: space::Pos,
    pub delta: space::Pos,
}

impl Block {
    pub fn compile(&self) -> CompiledBlock {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();

        let mut string_mode = false;

        let fun = prologue!(ops);
        for c in self.code.chars() {
            match c {
                '"' => string_mode = !string_mode,

                c if string_mode => {
                    funjit_dynasm!(ops ; mov rsi, QWORD c as _);
                    call_external!(ops, eval::Eval::push);
                }

                c @ '0'..='9' => {
                    let val = c as isize - '0' as isize;
                    funjit_dynasm!(ops ; mov rsi, QWORD val as _);
                    call_external!(ops, eval::Eval::push);
                }

                // would be nice to enforce that this is also the end of the instruction stream
                '@' => break,

                ',' => call_external!(ops, eval::Eval::output),
                '.' => call_external!(ops, eval::Eval::output_number),
                '~' => call_external!(ops, eval::Eval::input),

                ':' => {
                    call_external!(ops, eval::Eval::peek);
                    funjit_dynasm!(ops ; mov rsi, rax);
                    call_external!(ops, eval::Eval::push);
                }

                '+' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; add rsi, rax);
                    call_external!(ops, eval::Eval::push);
                }

                '-' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; sub rsi, rax);
                    call_external!(ops, eval::Eval::push);
                }

                '*' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; imul rsi, rax);
                    call_external!(ops, eval::Eval::push);
                }

                '$' => {
                    call_external!(ops, eval::Eval::pop);
                }

                _ => (),
            }
        }

        set_pc!(ops, self.pc);

        if self.loops {
            funjit_dynasm!(ops
                ; lea rax, [->entry]
                ; jmp rax
            );
        } else {
            epilogue!(ops, self.terminates);
        }

        let buffer = ops.finalize().unwrap();
        let code = unsafe { std::mem::transmute(buffer.ptr(fun)) };

        CompiledBlock {
            _buffer: buffer,
            code,
        }
    }
}

pub struct CompiledBlock {
    _buffer: dynasmrt::mmap::ExecutableBuffer,
    code: extern "sysv64" fn(&mut eval::Eval) -> bool,
}

impl CompiledBlock {
    pub fn run(&self, eval: &mut eval::Eval) -> bool {
        (self.code)(eval)
    }
}

pub struct Jit {}

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

                '#' => pc += &delta,

                ' ' => (),

                '@' => {
                    block.terminates = true;
                    break;
                }

                c => block.code.push(c),
            }

            pc += &delta;

            if seen.contains(&pc) {
                block.loops = true;
                break;
            }

            seen.insert(pc);
        }

        block.delta = delta;
        block.pc = pc;

        block
    }

    pub fn run(&mut self, eval: &mut eval::Eval) {
        let mut blocks: HashMap<space::Pos, CompiledBlock> = HashMap::new();

        loop {
            if !blocks.contains_key(&eval.pc) {
                let block = Jit::next_block(&eval.cells, eval.pc, eval.delta);
                blocks.insert(eval.pc, block.compile());
            }

            let compiled_block = blocks.get(&eval.pc).unwrap();
            if compiled_block.run(eval) {
                break;
            }

            // at this point we should be at a control instruction, so update delta and take a step
            // to find the next sequence.
            match eval.cells.get(eval.pc) {
                '|' => {
                    if eval.pop() == 0 {
                        eval.delta = space::Pos::south();
                    } else {
                        eval.delta = space::Pos::north();
                    }
                }

                '_' => {
                    if eval.pop() == 0 {
                        eval.delta = space::Pos::east()
                    } else {
                        eval.delta = space::Pos::west()
                    }
                }

                c => {
                    println!("Unhandled control instruction: {}\n", c);
                    break;
                }
            }

            eval.pc += &eval.delta;
        }
    }
}