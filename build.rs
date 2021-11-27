
extern crate anyhow;

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const TEST_PREFIX: &str = "
use super::jit;
use super::space;
";

// TODO: this currently won't cause a failure if tests fail to terminate, and will just hang
// instead.
const TEST_TEMPLATE: &str = "
#[test]
fn test_%PREFIX%() {
    let prog = std::fs::read_to_string(\"%ROOT%/tests/%PREFIX%.bf\").expect(\"Failed to read test file\");
    let mut jit = jit::Jit::new(space::Funge93::from_string(&prog));
    jit.run();
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
