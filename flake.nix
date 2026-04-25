{
  description = "forge — tmux sessionizer with includes and overseer integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-mozilla, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ nixpkgs-mozilla.overlay ];
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

        # Home-manager module for use in user configurations
        # Usage in home-manager flake:
        #   imports = [ inputs.forge.homeManagerModules.${system} ];
        homeManagerModules = import ./module {
          lib = nixpkgs.lib;
          inherit pkgs;
          forge = self.packages.${system}.default;
        };
      }
    );
}