
extern crate clap;
extern crate rand;

use clap::{Arg, App};

mod space;
mod eval;

fn main() {
    let matches = App::new("funjit")
        .version("1.0")
        .arg(Arg::with_name("INPUT")
             .required(true)
             .index(1))
        .get_matches();

    let file = matches.value_of("INPUT").unwrap();

    let prog = std::fs::read_to_string(file).expect("Failed to read test.bf");
    eval::Eval::new(space::Funge93::from_string(&prog)).run();
}
