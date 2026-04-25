name="rust"
desc="Rust project with cargo and rustflake"
path="Code/Rust"
direnv="use_flake"
requires=["cargo", "rustflake"]
setup_priority="10"

build="cargo build"
run="cargo run"
test="cargo test"
check="cargo check"
