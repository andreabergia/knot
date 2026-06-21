fn main() {
    let expected: &[&str] = &["ts-debugger", "ts-console", "py-mutable-default-arg"];

    for name in expected {
        let path = format!("rules/build/{name}.wasm");
        println!("cargo::rerun-if-changed={path}");

        if !std::path::Path::new(&path).exists() {
            panic!(
                "bundled rule `{name}` not found at `{path}`.\n\
                 Run `cargo run -p knot-xtask -- build-rules` first."
            );
        }
    }
}