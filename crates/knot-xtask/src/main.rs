use std::{fs, path::PathBuf, process::Command};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build-rules") => build_rules(),
        other => {
            eprintln!(
                "usage: cargo run -p knot-xtask -- build-rules (got {:?})",
                other
            );
            std::process::exit(1);
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn build_rules() -> anyhow::Result<()> {
    let workspace = workspace_root();
    let rules_manifest = workspace.join("rules/Cargo.toml");

    println!("building rules workspace: {}", rules_manifest.display());

    let status = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            rules_manifest.to_str().unwrap(),
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
        .status()?;

    if !status.success() {
        anyhow::bail!("rules build failed");
    }

    let wasm_dir = workspace.join("rules/target/wasm32-unknown-unknown/release");
    let output_dir = workspace.join("crates/knot-core/rules/build");
    fs::create_dir_all(&output_dir)?;

    let rule_ids = ["ts-debugger", "ts-console", "py-mutable-default-arg"];

    for id in &rule_ids {
        let wasm_name = format!("{id}.wasm");
        let src = wasm_dir.join(&wasm_name);

        if !src.exists() {
            anyhow::bail!("expected wasm artifact not found: {}", src.display());
        }

        let dst = output_dir.join(&wasm_name);
        fs::copy(&src, &dst)?;
        println!("  copied {wasm_name}");
    }

    println!(
        "done — {} rules written to {}",
        rule_ids.len(),
        output_dir.display()
    );
    Ok(())
}
