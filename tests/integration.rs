use std::process::{Command};

use test_generator::test_resources;

#[test_resources("tests/*.js")]
pub fn js(test: &str) {
    let output = Command::new("node")
        .current_dir("./tests")
        .arg("--experimental-wasm-modules")
        .arg(test.replace("tests/", ""))
        .output()
        .expect("failed to execute process");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    if !output.status.success() {
        panic!("stdout from node: \"{}\"\nstderr from node: \"{}\"", stdout, stderr);
    }
}