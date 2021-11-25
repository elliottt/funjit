use rand;
use std::io::{self, BufReader, prelude::*};

use super::space;

pub struct Eval {
    pub cells: space::Funge93,
    pub input: BufReader<Box<dyn Read>>,
    pub output: Box<dyn Write>,
    pub stack: Vec<isize>,
    pub pc: space::Pos,
    pub delta: space::Pos,
}

impl Eval {
    pub fn new(cells: space::Funge93) -> Self {
        Eval {
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
        if y >= 0 && y < space::Funge93::HEIGHT as isize && x >= 0 && x < space::Funge93::WIDTH as isize {
            self.cells.get(space::Pos::new(x,y)) as isize
        } else {
            0
        }
    }

    pub fn put(&mut self) {
        let y = self.pop();
        let x = self.pop();
        let v = self.pop();
        if y >= 0 && y < space::Funge93::HEIGHT as isize && x >= 0 && x < space::Funge93::WIDTH as isize {
            let cell = self.cells.get_mut(space::Pos::new(x,y));
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

    #[allow(dead_code)]
    pub fn run(&mut self) {
        let mut pc = space::Pos::new(0, 0);
        let mut delta = space::Pos::new(1, 0);
        let mut string_mode = false;
        loop {
            if string_mode {
                match *self.cells.get_mut(pc) {
                    '"' => string_mode = false,
                    c => self.push(c as isize),
                }
            } else {
                match *self.cells.get_mut(pc) {
                    '@' => break,

                    '"' => string_mode = true,

                    // stack operations
                    ':' => {
                        let val = self.peek();
                        self.push(val)
                    }
                    '\\' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(b);
                        self.push(a)
                    }

                    '$' => {
                        self.pop();
                    }

                    // input
                    '&' => {
                        let mut text = String::new();
                        std::io::stdin()
                            .read_line(&mut text)
                            .expect("Failed to read a line");
                        let num = text
                            .trim()
                            .parse::<isize>()
                            .expect("Failed to read a number");
                        self.push(num);
                    }

                    '~' => {
                        let mut buf = [0; 1];
                        std::io::stdin()
                            .read_exact(&mut buf)
                            .expect("Failed to read a line");
                        self.push(buf[0] as isize);
                    }

                    // output
                    '.' => print!("{} ", self.pop()),
                    ',' => print!("{}", std::char::from_u32(self.pop() as u32).unwrap()),

                    // control flow
                    '_' => {
                        if self.pop() == 0 {
                            delta = space::Pos::east()
                        } else {
                            delta = space::Pos::west()
                        }
                    }

                    '|' => {
                        if self.pop() == 0 {
                            delta = space::Pos::south()
                        } else {
                            delta = space::Pos::north()
                        }
                    }
                    '#' => pc += &delta,
                    '?' => match rand::random::<usize>() % 4 {
                        0 => delta = space::Pos::north(),
                        1 => delta = space::Pos::east(),
                        2 => delta = space::Pos::south(),
                        _ => delta = space::Pos::west(),
                    },

                    // delta operations
                    '^' => delta = space::Pos::north(),
                    '>' => delta = space::Pos::east(),
                    'v' => delta = space::Pos::south(),
                    '<' => delta = space::Pos::west(),

                    // arithmetic
                    '+' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(a + b)
                    }
                    '-' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(a - b)
                    }
                    '*' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(a * b)
                    }
                    '/' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(a / b)
                    }
                    '%' => {
                        let a = self.pop();
                        let b = self.pop();
                        self.push(a % b)
                    }

                    '!' => {
                        if self.pop() == 0 {
                            self.push(1)
                        } else {
                            self.push(0)
                        }
                    }

                    '`' => {
                        let a = self.pop();
                        let b = self.pop();
                        if a > b {
                            self.push(1)
                        } else {
                            self.push(0)
                        }
                    }

                    // metaprogramming
                    'p' => {
                        let y = self.pop();
                        let x = self.pop();
                        let c = self.pop();
                        let cell = self.cells.get_mut(space::Pos { x, y });
                        *cell = char::from_u32(c as u32).unwrap();
                    }

                    'g' => {
                        let y = self.pop();
                        let x = self.pop();
                        let c = *self.cells.get_mut(space::Pos { x, y });
                        self.push(c as isize);
                    }

                    // numeric literals
                    c @ '0'..='9' => self.push(c as isize - '0' as isize),

                    ' ' => (),

                    c => panic!("Unknown opcode {}\n", c),
                }
            }

            pc += &delta;
        }
    }
}
