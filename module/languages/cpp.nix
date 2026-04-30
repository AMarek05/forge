{
  description = "C++ project with cmake";
  path = "Code/C++";
  direnv = "use flake";
  buildInputs = [
    "cmake"
    "clang"
  ];
}