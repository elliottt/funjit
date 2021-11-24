
extern crate anyhow;
extern crate clap;
extern crate rand;
extern crate dynasm;
extern crate dynasmrt;

use clap::{Arg, App};

mod space;
mod eval;
mod jit;

fn main() -> Result<(), anyhow::Error> {
    let matches = App::new("funjit")
        .version("1.0")
        .arg(Arg::with_name("INPUT")
             .required(true)
             .index(1))
        .get_matches();

    let file = matches.value_of("INPUT").unwrap();

    let prog = std::fs::read_to_string(file).expect("Failed to read test.bf");
    let jit = jit::Jit::new()?;

    let space = space::Funge93::from_string(&prog);
    let block = jit::Jit::next_block(&space, space::Pos::new(0, 0), space::Pos::east());

    let compiled_block = block.compile();

    let mut eval = eval::Eval::new(space);

    let res = compiled_block.run(&mut eval);

    eval.output.flush();
    println!("");

    Ok(())
}
