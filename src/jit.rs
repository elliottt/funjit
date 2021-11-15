use mmap_rs::{Mmap, MmapOptions};
use std::collections::HashSet;

use super::space;

#[derive(Default)]
pub struct Block {
    pub code: String,
    pub loops: bool,
    pub mutates: bool,
}

pub struct Jit {
    code: Option<Mmap>,
}

impl Jit {
    pub fn new() -> Result<Self, anyhow::Error> {
        let code = unsafe {
            MmapOptions::new()
                .with_size(4096)
                // .map_mut()
                .map_exec()
        }?;

        Ok(Jit { code: Some(code) })
    }

    // Returns basic blocks from the funge space
    pub fn next_block(space: &space::Funge93, mut pc: space::Pos, mut delta: space::Pos) -> Block {
        let start = pc;
        let mut block = Block::default();

        loop {
            match space.get(pc) {
                '_' | '|' | 'p' => break,

                'p' => block.mutates = true,

                '^' => delta = space::Pos::north(),
                '>' => delta = space::Pos::east(),
                'v' => delta = space::Pos::south(),
                '<' => delta = space::Pos::west(),

                ' ' => (),

                c => block.code.push(c),
            }

            pc += &delta;

            if pc == start {
                block.loops = true;
                break;
            }
        }

        block
    }

    // TODO
    pub fn compile(&mut self) -> Result<(), anyhow::Error> {
        let code = std::mem::replace(&mut self.code, None).unwrap();
        let code = code.make_mut().map_err(|(_, err)| err)?;
        // compile
        let code = code.make_exec().map_err(|(_, err)| err)?;
        std::mem::replace(&mut self.code, Some(code));
        Ok(())
    }
}
