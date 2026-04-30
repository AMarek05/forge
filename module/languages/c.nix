{
  description = "C project with gcc and make";
  path = "Code/C";
  direnv = "use flake";
  buildInputs = [
    "gcc"
    "make"
  ];
}