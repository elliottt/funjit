#[allow(unused_imports)]
use dynasmrt::mmap::ExecutableBuffer;

use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use std::collections::{HashMap, HashSet};
use std::io::{self, prelude::*};

use super::space;

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
    ($ops:ident, $i:ident, $pc:expr) => {
        funjit_dynasm!($ops ; mov rsi, QWORD $pc.x as _);
        funjit_dynasm!($ops ; mov rdx, QWORD $pc.y as _);
        call_external!($ops, Jit::<$i>::set_pc);
    }
}

macro_rules! set_delta {
    ($ops:ident, $i:ident, $pc:expr) => {
        funjit_dynasm!($ops ; mov rsi, QWORD $pc.x as _);
        funjit_dynasm!($ops ; mov rdx, QWORD $pc.y as _);
        call_external!($ops, Jit::<$i>::set_delta);
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
    ($ops:ident, $i:ident) => {
        call_external!($ops, Jit::<$i>::pop);
        funjit_dynasm!($ops ; mov [rsp + 8], rax);
        call_external!($ops, Jit::<$i>::pop);
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
    pub fn compile<I: IO>(&self) -> CompiledBlock<I> {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();

        let mut string_mode = false;

        let fun = prologue!(ops);
        for c in self.code.chars() {
            match c {
                '"' => string_mode = !string_mode,

                c if string_mode => {
                    funjit_dynasm!(ops ; mov rsi, QWORD c as _);
                    call_external!(ops, Jit::<I>::push);
                }

                c @ '0'..='9' => {
                    let val = c as isize - '0' as isize;
                    funjit_dynasm!(ops ; mov rsi, QWORD val as _);
                    call_external!(ops, Jit::<I>::push);
                }

                // would be nice to enforce that this is also the end of the instruction stream
                '@' => break,

                ',' => call_external!(ops, Jit::<I>::output),
                '.' => call_external!(ops, Jit::<I>::output_number),
                '~' => call_external!(ops, Jit::<I>::input),
                '&' => call_external!(ops, Jit::<I>::input_number),

                ':' => {
                    call_external!(ops, Jit::<I>::peek);
                    funjit_dynasm!(ops ; mov rsi, rax);
                    call_external!(ops, Jit::<I>::push);
                }

                '\\' => {
                    call_external!(ops, Jit::<I>::pop);
                    funjit_dynasm!(ops ; mov [rsp + 8], rax);
                    call_external!(ops, Jit::<I>::pop);
                    funjit_dynasm!(ops
                        ; mov rsi, [rsp + 8]
                        ; mov [rsp + 8], rax
                    );
                    call_external!(ops, Jit::<I>::push);
                    funjit_dynasm!(ops ; mov rsi, [rsp + 8]);
                    call_external!(ops, Jit::<I>::push);
                }

                '!' => {
                    call_external!(ops, Jit::<I>::pop);
                    funjit_dynasm!(ops
                        ; mov rsi, QWORD 0
                        ; cmp rax, rsi
                        ; jne >write
                        ; inc rsi
                        ; write:
                    );
                    call_external!(ops, Jit::<I>::push);
                }

                '`' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops
                        ; cmp rax, rsi
                        ; mov rsi, QWORD 0
                        ; jle >write
                        ; inc rsi
                        ; write:
                    );
                    call_external!(ops, Jit::<I>::push);
                }

                'g' => {
                    call_external!(ops, Jit::<I>::get);
                    funjit_dynasm!(ops ; mov rsi, rax);
                    call_external!(ops, Jit::<I>::push);
                }

                '+' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops ; add rsi, rax);
                    call_external!(ops, Jit::<I>::push);
                }

                '-' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops ; sub rsi, rax);
                    call_external!(ops, Jit::<I>::push);
                }

                '*' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops ; imul rsi, rax);
                    call_external!(ops, Jit::<I>::push);
                }

                '/' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops
                        ; xor rdx, rdx
                        ; idiv rsi
                        ; mov rsi, rax
                    );
                    call_external!(ops, Jit::<I>::push);
                }

                '%' => {
                    binop!(ops, I);
                    funjit_dynasm!(ops
                        ; xor rdx, rdx
                        ; idiv rsi
                        ; mov rsi, rdx
                    );
                    call_external!(ops, Jit::<I>::push);
                }

                '$' => call_external!(ops, Jit::<I>::pop),

                _ => {
                    println!("Unhandled instruction: {}\n", c);
                    break;
                }
            }
        }

        set_pc!(ops, I, self.pc);
        set_delta!(ops, I, self.delta);

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

pub struct CompiledBlock<I: IO> {
    _buffer: dynasmrt::mmap::ExecutableBuffer,
    code: extern "sysv64" fn(&mut Jit<I>) -> bool,
}

impl<I: IO> CompiledBlock<I> {
    pub fn run(&self, state: &mut Jit<I>) -> bool {
        (self.code)(state)
    }
}

pub trait IO {
    fn input_char(&mut self) -> Option<u8>;
    fn input_number(&mut self) -> isize;
    fn output_char(&mut self, c: u8);
    fn output_number(&mut self, n: isize);
}

pub struct StdIO {
    input: std::io::Stdin,
    output: std::io::Stdout,
}

impl StdIO {
    pub fn new() -> Self {
        StdIO {
            input: io::stdin(),
            output: io::stdout(),
        }
    }
}

impl IO for StdIO {
    fn input_char(&mut self) -> Option<u8> {
        let mut buf = [0; 1];

        // NOTE: unwrap might cause issues here
        if let Ok(()) = self.input.read_exact(&mut buf) {
            Some(buf[0])
        } else {
            None
        }
    }

    fn input_number(&mut self) -> isize {
        let mut text = String::new();
        self.input
            .read_line(&mut text)
            .expect("Failed to read a line");
        text.trim()
            .parse::<isize>()
            .expect("Failed to read a number")
    }

    fn output_char(&mut self, c: u8) {
        let buf = [c; 1];
        self.output.write_all(&buf).unwrap();
        self.output.flush().unwrap();
    }

    fn output_number(&mut self, n: isize) {
        self.output.write_fmt(format_args!("{}", n)).unwrap();
        self.output.flush().unwrap();
    }
}

pub struct Jit<I: IO> {
    pub cells: space::Funge93,
    pub io: I,
    pub stack: Vec<isize>,
    pub pc: space::Pos,
    pub delta: space::Pos,
}

impl<I: IO> Jit<I> {
    pub fn new(cells: space::Funge93, io: I) -> Self {
        Jit {
            cells,
            io,
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
            self.cells.get(x as usize, y as usize) as isize
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
            self.cells.set(x as usize, y as usize, v as u8);
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
        if let Some(c) = self.io.input_char() {
            self.push(c as isize)
        } else {
            self.push(-1);
        }
    }

    pub fn output(&mut self) {
        let val = self.pop();
        self.io.output_char(val as u8);
    }

    pub fn input_number(&mut self) {
        let num = self.io.input_number();
        self.push(num);
    }

    pub fn output_number(&mut self) {
        let val = self.pop();
        self.io.output_number(val);
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

    // Returns basic blocks from the funge space
    pub fn next_block(space: &space::Funge93, mut pc: space::Pos, mut delta: space::Pos) -> Block {
        let mut block = Block::default();
        let mut seen = HashSet::new();
        let mut string_mode = false;

        loop {
            match space.get(pc.x as usize, pc.y as usize) {
                c if string_mode => {
                    if c == b'"' {
                        string_mode = !string_mode
                    }
                    block.code.push(c as char)
                }

                b'_' | b'|' | b'?' => break,

                b'p' => {
                    block.mutates = true;
                    break;
                }

                b'@' => {
                    block.terminates = true;
                    break;
                }

                b'^' => delta = space::Pos::north(),
                b'>' => delta = space::Pos::east(),
                b'v' => delta = space::Pos::south(),
                b'<' => delta = space::Pos::west(),

                b'#' => pc.move_by(&delta),

                b' ' if !string_mode => (),

                c => {
                    if c == b'"' {
                        string_mode = !string_mode
                    }
                    block.code.push(c as char)
                }
            }

            pc.move_by(&delta);

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

    pub fn run(&mut self) {
        let mut blocks: HashMap<space::Pos, CompiledBlock<I>> = HashMap::new();

        loop {
            // at this point we should be at a control instruction, so update delta and take a step
            // to find the next sequence.
            match self.cells.get(self.pc.x as usize, self.pc.y as usize) {
                b'|' => {
                    if self.pop() == 0 {
                        self.delta = space::Pos::south();
                    } else {
                        self.delta = space::Pos::north();
                    }
                }

                b'_' => {
                    if self.pop() == 0 {
                        self.delta = space::Pos::east()
                    } else {
                        self.delta = space::Pos::west()
                    }
                }

                b'?' => match rand::random::<usize>() % 4 {
                    0 => self.delta = space::Pos::north(),
                    1 => self.delta = space::Pos::east(),
                    2 => self.delta = space::Pos::south(),
                    _ => self.delta = space::Pos::west(),
                },

                b'p' => {
                    blocks.clear();
                    self.put();
                }

                // everything else should be compiled
                _ => {
                    if !blocks.contains_key(&self.pc) {
                        // NOTE: there's no special handling for when the blocks are empty, as the
                        // compiled function will end up setting the pc and delta. This happens
                        // when a block is made up entirely of instructions that change the
                        // direction of the cursor, or whitespace.
                        let block = Self::next_block(&self.cells, self.pc, self.delta);
                        blocks.insert(self.pc, block.compile());
                    }

                    let compiled_block = blocks.get(&self.pc).unwrap();
                    if compiled_block.run(self) {
                        break;
                    }

                    // no need to update pc, the compiled function does that
                    continue;
                }
            }

            self.pc.move_by(&self.delta);
        }
    }
}
