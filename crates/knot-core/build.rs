use std::{env, fs, path::PathBuf};

fn main() {
    let source = "rules/ts-debugger.wat";
    println!("cargo::rerun-if-changed={source}");

    let wasm = wat::parse_file(source).expect("bundled ts-debugger rule should compile");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"))
        .join("ts-debugger.wasm");
    fs::write(output, wasm).expect("bundled ts-debugger rule should be written");
}
