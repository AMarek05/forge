{ lib, pkgs, runCommand ? pkgs.runCommand }:

let
  languages-list = [ "rust" "python" "c" "cpp" "java" "nix" "r" "txt" ];

  # Load language definitions from individual files
  all-languages = lib.mapAttrs (name: _: import (./languages + "/${name}.nix"))
    (lib.genAttrs languages-list (name: name));

  default-languages = languages-list;

  # ─── Generation ─────────────────────────────────────────────────────────────

  generate-lang-flake =
    name: lang:
    let
      bi = toString lang.buildInputs;
    in
    builtins.toFile "flake.nix" ''
      {
        description = "${lang.description}";
        inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        outputs = { self, nixpkgs }:
          let
            system = "x86_64-linux";
            pkgs = import nixpkgs { inherit system; };
          in
          {
            devShells.''${system}.default = pkgs.mkShell {
              name = "${lang.description}";
              buildInputs = with pkgs; [ ${bi} ];
            };
          };
      }
    '';

  generate-lang-setup =
    name: lang:
    let
      setup-commands =
        if name == "rust" then
          ''
            mkdir -p "$FORGE_PROJECT_PATH"
            cd "$FORGE_PROJECT_PATH"
            render_template "$FORGE_LANG_DIR/${name}/flake.nix" flake.nix
            if [ ! -d .git ]; then git init; git add flake.nix; git commit -m "init"; fi
            cat > .envrc << 'ENVEOF'
            use flake
            ENVEOF
            if [ ! -f Cargo.toml ]; then nix develop . -c cargo init . || true; fi
            direnv allow
          ''
        else if name == "python" then
          ''
            mkdir -p "$FORGE_PROJECT_PATH"
            cd "$FORGE_PROJECT_PATH"
            render_template "$FORGE_LANG_DIR/${name}/flake.nix" flake.nix
            if [ ! -d .git ]; then git init; git add flake.nix; git commit -m "init"; fi
            cat > .envrc << 'ENVEOF'
            use flake
            ENVEOF
            if [ ! -f pyproject.toml ]; then nix develop . -c poetry init --name "$FORGE_PROJECT_NAME" --quiet || true; fi
            direnv allow
          ''
        else
          ''
            mkdir -p "$FORGE_PROJECT_PATH"
            cd "$FORGE_PROJECT_PATH"
            render_template "$FORGE_LANG_DIR/${name}/flake.nix" flake.nix
            if [ ! -d .git ]; then git init; git add flake.nix; git commit -m "init"; fi
            cat > .envrc << 'ENVEOF'
            use flake
            ENVEOF
            direnv allow
          '';
    in
    builtins.toFile "setup.sh" ''
      #!/bin/bash
      set -e
      render_template() { sed "s/{{PROJECT_NAME}}/$FORGE_PROJECT_NAME/g" "$1" > "$2"; }
      if [ "$FORGE_DRY_RUN" = "1" ]; then echo "[dry-run] setup"; exit 0; fi
      ${setup-commands}
    '';

  generate-lang-langwl =
    name: lang:
    builtins.toFile "lang.wl" ''
      name="${name}"
      desc="${lang.description}"
      path="${lang.path}"
      direnv="${lang.direnv}"
      build=""
      run=""
      test=""
      check=""
    '';

  lang-files = builtins.mapAttrs (name: lang: {
    flake_nix = generate-lang-flake name lang;
    setup_sh  = generate-lang-setup name lang;
    lang_wl   = generate-lang-langwl name lang;
  }) all-languages;

  # ─── Output directory ───────────────────────────────────────────────────────
  lang-dir = runCommand "forge-languages" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    mkdir -p $out/default $out/custom
    ${lib.concatMapStrings (lang: ''
      mkdir -p $out/default/${lang}
      cp ''${lang-files.${lang}.flake_nix} $out/default/${lang}/flake.nix
      cp ''${lang-files.${lang}.setup_sh}  $out/default/${lang}/setup.sh
      cp ''${lang-files.${lang}.lang_wl}   $out/default/${lang}/lang.wl
    '') default-languages}
  '';
in
{
  inherit all-languages default-languages lang-files lang-dir;
}