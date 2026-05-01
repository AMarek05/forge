{ lib, pkgs, runCommand ? pkgs.runCommand }:

let
  includes-list = [ "git" "overseer" ];

  # Load include definitions from individual files
  all-includes = lib.mapAttrs (name: _: import (./includes + "/${name}.nix"))
    (lib.genAttrs includes-list (name: name));

  default-includes = includes-list;

  # ─── Generation ─────────────────────────────────────────────────────────────

  generate-include-includewl =
    name: inc:
    builtins.toFile "include.wl" (
      builtins.toJSON {
        inherit (inc) description provides;
        version = "1.0";
      }
    );

  generate-include-setup =
    name: inc:
    builtins.toFile "setup.sh" ''
      #!/bin/bash
      set -e
      if [ "$FORGE_DRY_RUN" = "1" ]; then echo "[dry-run] ${name} include"; exit 0; fi
      ${inc.setup}
    '';

  include-files = builtins.mapAttrs (name: inc: {
    include_wl = generate-include-includewl name inc;
    setup_sh   = generate-include-setup name inc;
  }) all-includes;

  # ─── Output directory ───────────────────────────────────────────────────────
  include-dir = runCommand "forge-includes" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    mkdir -p $out
    ${lib.concatMapStrings (inc: ''
      mkdir -p $out/${inc}
      cp ${include-files.${inc}.include_wl} $out/${inc}/include.wl
      cp ${include-files.${inc}.setup_sh}   $out/${inc}/setup.sh
    '') default-includes}
  '';
in
{
  inherit all-includes default-includes include-files include-dir;
}