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
    # 1. System-specific outputs (packages, shells, etc.)
    flake-utils.lib.eachDefaultSystem (
      system:
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
      }
    )
    # 2. Merge with system-agnostic outputs (modules)
    // {
      homeManagerModules.default =
        { pkgs, ... }:
        {
          imports = [ ./module ];

          _module.args.forgePkg = self.packages.${pkgs.system}.default;
        };
    };
}

