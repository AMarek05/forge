{
  description = "Nix flake project";
  path = "Code/Nix";
  direnv = "use flake";
  buildInputs = [ "nix" ];
}