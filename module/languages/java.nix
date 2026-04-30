{
  description = "Java project with maven";
  path = "Code/Java";
  direnv = "use flake";
  buildInputs = [
    "maven"
    "jdk17"
  ];
}