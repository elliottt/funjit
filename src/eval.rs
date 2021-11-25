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
}
