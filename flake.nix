{
  description = "forge — tmux sessionizer with includes and overseer integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay { inherit system; }) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        packages = {
          default = pkgs.callPackage ./nix/package.nix { };
          forge = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
            clippy
          ];
        };
      }
    );
}