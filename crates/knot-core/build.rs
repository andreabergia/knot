use std::{env, fs, path::PathBuf};

fn compile_wat(name: &str) {
    let source = format!("rules/{name}.wat");
    println!("cargo::rerun-if-changed={source}");

    let wasm = wat::parse_file(&source).expect("bundled {name} rule should compile");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"))
        .join(format!("{name}.wasm"));
    fs::write(output, wasm).expect("bundled {name} rule should be written");
}

fn main() {
    compile_wat("ts-debugger");
    compile_wat("ts-console");
    compile_wat("py-mutable-default-arg");
}
