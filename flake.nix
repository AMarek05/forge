{
  description = "forge — tmux sessionizer with includes and overseer integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      nixpkgs-mozilla,
      flake-utils,
    }:
    # 1. System-specific outputs
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ nixpkgs-mozilla.overlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        # Cleaned up package definition
        packages = rec {
          forge = pkgs.callPackage ./nix/package.nix { };
          default = forge;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.forge ];
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
            clippy
          ];
        };
      }
    )
    // {
      # 2. System-agnostic Home Manager Module
      homeManagerModules.default =
        { pkgs, ... }:
        {
          imports = [ ./module ]; # Points to your module.nix file/directory

          # We set the default value for the option directly in the config
          # to ensure it's "batteries included"
          forge.package = nixpkgs.lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.forge;
        };
    };
}
