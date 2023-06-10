extern crate anyhow;
extern crate clap;
extern crate dynasm;
extern crate dynasmrt;
extern crate rand;

#[cfg(test)]
pub mod test {
    include!(concat!(env!("OUT_DIR"), "/exp_tests.rs"));
}

use clap::{App, Arg};

mod jit;
mod space;

fn main() -> Result<(), anyhow::Error> {
    let matches = App::new("funjit")
        .version("1.0")
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .get_matches();

    let file = matches.value_of("INPUT").unwrap();

    let prog = std::fs::read_to_string(file)?;
    let mut jit = jit::Jit::new(space::Funge93::from_string(&prog), jit::StdIO::new());

    jit.run();

    Ok(())
}
