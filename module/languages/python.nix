{
  description = "Python project with poetry";
  path = "Code/Python";
  direnv = "use flake";
  buildInputs = [
    "python311"
    "poetry"
  ];
}