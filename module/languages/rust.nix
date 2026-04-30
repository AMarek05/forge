{
  description = "Rust project with cargo";
  path = "Code/Rust";
  direnv = "use flake";
  buildInputs = [
    "rustc"
    "cargo"
    "rustfmt"
    "clippy"
  ];
}