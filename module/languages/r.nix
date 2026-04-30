{
  description = "R project with renv";
  path = "Code/R";
  direnv = "use flake";
  buildInputs = [
    "R"
    "renv"
  ];
}