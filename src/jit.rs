#[allow(unused_imports)]
use dynasmrt::mmap::ExecutableBuffer;

use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use std::collections::{HashMap, HashSet};
use std::io::{self, prelude::*, BufReader};

use super::space;

pub struct State {
    cells: space::Funge93,
    input: BufReader<Box<dyn Read>>,
    output: Box<dyn Write>,
    stack: Vec<isize>,
    pc: space::Pos,
    delta: space::Pos,
}

impl State {
    pub fn new(cells: space::Funge93) -> Self {
        State {
            cells,
            input: BufReader::new(Box::new(io::stdin())),
            output: Box::new(io::stdout()),
            stack: Vec::new(),
            pc: space::Pos::new(0, 0),
            delta: space::Pos::new(1, 0),
        }
    }

    pub fn get(&mut self) -> isize {
        let y = self.pop();
        let x = self.pop();
        if y >= 0
            && y < space::Funge93::HEIGHT as isize
            && x >= 0
            && x < space::Funge93::WIDTH as isize
        {
            self.cells.get(space::Pos::new(x, y)) as isize
        } else {
            0
        }
    }

    pub fn put(&mut self) {
        let y = self.pop();
        let x = self.pop();
        let v = self.pop();
        if y >= 0
            && y < space::Funge93::HEIGHT as isize
            && x >= 0
            && x < space::Funge93::WIDTH as isize
        {
            let cell = self.cells.get_mut(space::Pos::new(x, y));
            *cell = char::from_u32(v as u32).unwrap();
        }
    }

    pub fn push(&mut self, val: isize) {
        self.stack.push(val)
    }

    pub fn set_pc(&mut self, x: isize, y: isize) {
        self.pc.x = x;
        self.pc.y = y;
    }

    pub fn set_delta(&mut self, x: isize, y: isize) {
        self.delta.x = x;
        self.delta.y = y;
    }

    pub fn input(&mut self) {
        let mut buf = [0; 1];

        // NOTE: unwrap might cause issues here
        self.input.read(&mut buf).unwrap();
        self.push(buf[0] as isize);
    }

    pub fn output(&mut self) {
        let val = self.pop();
        let buf = [val as u8; 1];
        self.output.write_all(&buf).unwrap();
        self.output.flush().unwrap();
    }

    pub fn input_number(&mut self) {
        let mut text = String::new();
        self.input
            .read_line(&mut text)
            .expect("Failed to read a line");
        let num = text
            .trim()
            .parse::<isize>()
            .expect("Failed to read a number");
        self.push(num);
    }

    pub fn output_number(&mut self) {
        let val = self.pop();
        self.output.write_fmt(format_args!("{}", val)).unwrap();
        self.output.flush().unwrap();
    }

    pub fn pop(&mut self) -> isize {
        if let Some(val) = self.stack.pop() {
            val
        } else {
            0
        }
    }

    pub fn peek(&mut self) -> isize {
        if let Some(val) = self.stack.last() {
            *val
        } else {
            0
        }
    }
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
        call_external!($ops, State::set_pc);
    }
}

macro_rules! set_delta {
    ($ops:ident, $pc:expr) => {
        funjit_dynasm!($ops ; mov rsi, QWORD $pc.x as _);
        funjit_dynasm!($ops ; mov rdx, QWORD $pc.y as _);
        call_external!($ops, State::set_delta);
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

// a b --
// rsi = a
// rax = b
macro_rules! binop {
    ($ops:ident) => {
        call_external!($ops, State::pop);
        funjit_dynasm!($ops ; mov [rsp + 8], rax);
        call_external!($ops, State::pop);
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
                    call_external!(ops, State::push);
                }

                c @ '0'..='9' => {
                    let val = c as isize - '0' as isize;
                    funjit_dynasm!(ops ; mov rsi, QWORD val as _);
                    call_external!(ops, State::push);
                }

                // would be nice to enforce that this is also the end of the instruction stream
                '@' => break,

                ',' => call_external!(ops, State::output),
                '.' => call_external!(ops, State::output_number),
                '~' => call_external!(ops, State::input),
                '&' => call_external!(ops, State::input_number),

                ':' => {
                    call_external!(ops, State::peek);
                    funjit_dynasm!(ops ; mov rsi, rax);
                    call_external!(ops, State::push);
                }

                '\\' => {
                    call_external!(ops, State::pop);
                    funjit_dynasm!(ops ; mov [rsp + 8], rax);
                    call_external!(ops, State::pop);
                    funjit_dynasm!(ops
                        ; mov rsi, [rsp + 8]
                        ; mov [rsp + 8], rax
                    );
                    call_external!(ops, State::push);
                    funjit_dynasm!(ops ; mov rsi, [rsp + 8]);
                    call_external!(ops, State::push);
                }

                '!' => {
                    call_external!(ops, State::pop);
                    funjit_dynasm!(ops
                        ; mov rsi, QWORD 0
                        ; cmp rax, rsi
                        ; jne >write
                        ; inc rsi
                        ; write:
                    );
                    call_external!(ops, State::push);
                }

                '`' => {
                    binop!(ops);
                    funjit_dynasm!(ops
                        ; cmp rax, rsi
                        ; mov rsi, QWORD 0
                        ; jle >write
                        ; inc rsi
                        ; write:
                    );
                    call_external!(ops, State::push);
                }

                'g' => {
                    call_external!(ops, State::get);
                    funjit_dynasm!(ops ; mov rsi, rax);
                    call_external!(ops, State::push);
                }

                '+' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; add rsi, rax);
                    call_external!(ops, State::push);
                }

                '-' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; sub rsi, rax);
                    call_external!(ops, State::push);
                }

                '*' => {
                    binop!(ops);
                    funjit_dynasm!(ops ; imul rsi, rax);
                    call_external!(ops, State::push);
                }

                '/' => {
                    binop!(ops);
                    funjit_dynasm!(ops
                        ; xor rdx, rdx
                        ; idiv rsi
                        ; mov rsi, rax
                    );
                    call_external!(ops, State::push);
                }

                '%' => {
                    binop!(ops);
                    funjit_dynasm!(ops
                        ; xor rdx, rdx
                        ; idiv rsi
                        ; mov rsi, rdx
                    );
                    call_external!(ops, State::push);
                }

                '$' => call_external!(ops, State::pop),

                _ => {
                    println!("Unhandled instruction: {}\n", c);
                    break;
                }
            }
        }

        set_pc!(ops, self.pc);
        set_delta!(ops, self.delta);

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
    code: extern "sysv64" fn(&mut State) -> bool,
}

impl CompiledBlock {
    pub fn run(&self, eval: &mut State) -> bool {
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

        loop {
            match space.get(pc) {
                '_' | '|' | '?' => break,

                'p' => {
                    block.mutates = true;
                    break;
                }

                '@' => {
                    block.terminates = true;
                    break;
                }

                '^' => delta = space::Pos::north(),
                '>' => delta = space::Pos::east(),
                'v' => delta = space::Pos::south(),
                '<' => delta = space::Pos::west(),

                '#' => pc += &delta,

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

        block.delta = delta;
        block.pc = pc;

        block
    }

    pub fn run(&mut self, eval: &mut State) {
        let mut blocks: HashMap<space::Pos, CompiledBlock> = HashMap::new();

        loop {
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

                '?' => match rand::random::<usize>() % 4 {
                    0 => eval.delta = space::Pos::north(),
                    1 => eval.delta = space::Pos::east(),
                    2 => eval.delta = space::Pos::south(),
                    _ => eval.delta = space::Pos::west(),
                },

                'p' => {
                    blocks.clear();
                    eval.put();
                }

                // everything else should be compiled
                _ => {
                    if !blocks.contains_key(&eval.pc) {
                        // NOTE: there's no special handling for when the blocks are empty, as the
                        // compiled function will end up setting the pc and delta. This happens
                        // when a block is made up entirely of instructions that change the
                        // direction of the cursor, or would
                        let block = Jit::next_block(&eval.cells, eval.pc, eval.delta);
                        blocks.insert(eval.pc, block.compile());
                    }

                    let compiled_block = blocks.get(&eval.pc).unwrap();
                    if compiled_block.run(eval) {
                        break;
                    }

                    // no need to update pc, the compiled function does that
                    continue;
                }
            }

            eval.pc += &eval.delta;
        }
    }
}
