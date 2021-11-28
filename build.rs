extern crate anyhow;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const TEST_PREFIX: &str = "

use std::io::{Cursor, Read, prelude::*};
use std::fs::File;

use super::jit;
use super::space;

struct BufferIO {
    input: Cursor<String>,
    output: Cursor<Vec<u8>>,
}

impl BufferIO {
    pub fn new() -> Self {
        BufferIO {
            input: Cursor::new(String::new()),
            output: Cursor::new(Vec::new()),
        }
    }
}

impl jit::IO for BufferIO {
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
            .expect(\"Failed to read a line\");
        text.trim()
            .parse::<isize>()
            .expect(\"Failed to read a number\")
    }

    fn output_char(&mut self, c: u8) {
        let buf = [c; 1];
        self.output.write_all(&buf).unwrap();
        self.output.flush().unwrap();
    }

    fn output_number(&mut self, n: isize) {
        self.output.write_fmt(format_args!(\"{}\", n)).unwrap();
        self.output.flush().unwrap();
    }
}

";

// TODO: this currently won't cause a failure if tests fail to terminate, and will just hang
// instead.
const TEST_TEMPLATE: &str = "
#[test]
fn test_%PREFIX%() {
    let prog = std::fs::read_to_string(\"%ROOT%/tests/%PREFIX%.bf\").expect(\"Failed to read test file\");

    let mut io = BufferIO::new();
    if let Ok(mut file) = File::open(\"%ROOT%/tests/%PREFIX%.bf.input\") {
        file.read_to_string(io.input.get_mut()).expect(\"Failed to read input\");
    }

    let mut jit = jit::Jit::new(space::Funge93::from_string(&prog), io);
    jit.run();

    if let Ok(mut file) = File::open(\"%ROOT%/tests/%PREFIX%.bf.output\") {
        let mut expected = String::new();
        file.read_to_string(&mut expected).unwrap();

        let actual = String::from_utf8(jit.io.output.get_ref().clone()).unwrap();
        assert!(expected == actual, \"{}\", colored_diff::PrettyDifference {
            expected: &expected,
            actual: &actual,
        });
    }
}
";

fn main() -> Result<(), anyhow::Error> {
    let out_dir = env::var("OUT_DIR")?;
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let dest = Path::new(&out_dir).join("exp_tests.rs");
    let mut test_file = File::create(&dest)?;

    writeln!(test_file, "{}", TEST_PREFIX)?;

    for exp in std::fs::read_dir("tests")? {
        let exp = exp?.path().canonicalize()?;
        let fname = exp.file_name().unwrap().to_string_lossy();
        if let Some(prefix) = fname.strip_suffix(".bf") {
            let test = TEST_TEMPLATE
                .replace("%FILE%", &fname)
                .replace("%PREFIX%", prefix)
                .replace("%ROOT%", &manifest_dir);
            writeln!(test_file, "{}", test)?;
        }
    }

    Ok(())
}
