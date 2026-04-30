# forge Home-manager module
# Generates language flakes and include scripts at module eval time,
# ships them to the Nix store, and sets up the forge binary wrapper.

{
  config,
  lib,
  pkgs,
  ...
}@args:

let
  cfg = config.forge;

  compDir = ".local/share/zsh/site-functions";

  # Avoid naming the module argument `forge` to prevent collisions with `config.forge`.
  # Pass your package via `extraSpecialArgs = { forgePkg = ...; }` in your flake setup.
  forge-binary = args.forgePkg or null;

  # ─── Language and include lists ─────────────────────────────────────────────
  # Add new languages/includes by creating a file in languages/ or includes/

  languages-list = [ "rust" "python" "c" "cpp" "java" "nix" "r" "txt" ];
  includes-list   = [ "git" "overseer" ];

  # Load language definitions from individual files
  all-languages = lib.mapAttrs (name: _: import (./languages + "/${name}.nix"))
    (lib.genAttrs languages-list (name: name));

  # Load include definitions from individual files
  all-includes = lib.mapAttrs (name: _: import (./includes + "/${name}.nix"))
    (lib.genAttrs includes-list (name: name));

  default-languages = languages-list;
  default-includes  = includes-list;

  # ─── Flake generation ────────────────────────────────────────────────────────

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
      # In Nix multiline strings, standard bash variables like $1 don't need escaping.
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

  lang-files = builtins.mapAttrs (name: lang: {
    flake_nix = generate-lang-flake name lang;
    setup_sh  = generate-lang-setup name lang;
    lang_wl   = generate-lang-langwl name lang;
  }) all-languages;

  include-files = builtins.mapAttrs (name: inc: {
    include_wl = generate-include-includewl name inc;
    setup_sh  = generate-include-setup name inc;
  }) all-includes;

  zsh-completion = builtins.replaceStrings [ "@JQ@" ] [ "${pkgs.jq}/bin/jq" ] (
    builtins.readFile ./completion.zsh
  );

  # ─── Lang dir: default/ (HM-managed store) + custom/ (HM-managed, user-editable) ───
  lang-dir  = pkgs.runCommand "forge-languages" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    mkdir -p $out/default $out/custom
    ${lib.concatMapStrings (lang: ''
      mkdir -p $out/default/${lang}
      cp ''${lang-files.${lang}.flake_nix} $out/default/${lang}/flake.nix
      cp ${lang-files.${lang}.setup_sh}  $out/default/${lang}/setup.sh
      cp ${lang-files.${lang}.lang_wl}   $out/default/${lang}/lang.wl
    '') default-languages}
  '';

  include-dir = pkgs.runCommand "forge-includes" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''\n    mkdir -p $out/default $out/custom
    ${lib.concatMapStrings (inc: ''\n      mkdir -p $out/default/${inc}
      cp ${include-files.${inc}.include_wl} $out/default/${inc}/include.wl
      cp ${include-files.${inc}.setup_sh}   $out/default/${inc}/setup.sh
    '') default-includes}\n  '';

in

{
  options.forge = {
    enable = lib.mkEnableOption "forge — tmux sessionizer";

    configDir = lib.mkOption {
      default = "${config.home.homeDirectory}/.forge";
      type = lib.types.path;
      description = "Directory where forge stores config and runtime state";
    };

    syncBase = lib.mkOption {
      default = "${config.home.homeDirectory}/sync";
      type = lib.types.path;
      description = "Root directory where projects live";
    };

    editor = lib.mkOption {
      default = "${pkgs.neovim}/bin/nvim";
      type = lib.types.str;
      description = "Editor binary";
    };

    githubUser = lib.mkOption {
      default = null;
      type = lib.types.nullOr lib.types.str;
      description = "GitHub username for git include";
    };

    tmuxBinary = lib.mkOption {
      default = "${pkgs.tmux}/bin/tmux";
      type = lib.types.str;
      description = "tmux binary path";
    };

    package = lib.mkOption {
      default = forge-binary;
      type = lib.types.nullOr lib.types.package;
      description = "forge binary package";
    };
  };

  config = lib.mkIf cfg.enable {

    home.packages = lib.mkIf (cfg.package != null) [ cfg.package ];

    home.file."${compDir}/_forge".text = zsh-completion;

    programs.zsh = {
      enable = true;
      initContent = lib.mkOrder 550 ''
        fpath=("${config.home.homeDirectory}/${compDir}" $fpath)
      '';
    };

    # Write config.json directly (not a store symlink)
    # lang_dir and include_dir point to local directories, not store paths
    home.file."${cfg.configDir}/config.json".text = builtins.toJSON {
      sync_base   = cfg.syncBase;
      editor      = cfg.editor;
      tmux_bin    = cfg.tmuxBinary;
      github_user = cfg.githubUser;
      lang_dir    = "${cfg.configDir}/langs";
      include_dir = "${cfg.configDir}/includes";
    };

    home.file."${cfg.configDir}/langs/default".source  = lang-dir;
    home.file."${cfg.configDir}/includes/default".source = include-dir;

    home.sessionVariables = {
      FORGE_CONFIG_DIR = cfg.configDir;
    };
  };
}