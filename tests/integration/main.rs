//! forge integration test runner
//!
//! Runs all test suites from this directory.
//! Call with:  cargo test --test integration
//! Or:        ./run.sh [unit|integration|shell|all]

use std::path::PathBuf;
use std::process::Command;
use std::env;

/// Forge binary path (built via `nix build .#forge`)
fn forge_bin() -> PathBuf {
    if let Ok(p) = env::var("FORGE_BIN") {
        return PathBuf::from(p);
    }
    let out = Command::new("nix")
        .args(["build", ".#forge", "--print-out-paths", "--quiet"])
        .output()
        .expect("nix build .#forge failed");
    let path = String::from_utf8(out.stdout).expect("non-utf8 in nix store path");
    let path = path.trim();
    PathBuf::from(format!("{}/bin/forge", path))
}

fn main() {
    let bin = forge_bin();
    println!("Using forge binary: {}", bin.display());

    // Delegate to the bash test runner for full integration
    let status = Command::new("bash")
        .args(["tests/run.sh", "all"])
        .current_dir(env::current_dir().unwrap())
        .status()
        .expect("failed to run tests");

    std::process::exit(status.code().unwrap_or(1));
}