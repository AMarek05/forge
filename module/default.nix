# forge Home-manager module
# Generates language flakes and include scripts at module eval time,
# ships them to the Nix store, and sets up the forge binary wrapper.

{ config, lib, pkgs, ... }:

let
  cfg = config.forge;

  # ─── Language definitions ───────────────────────────────────────────────────

  rust-lang = {
    description = "Rust project with cargo";
    path = "Code/Rust";
    direnv = "use flake";
    requires = [ "cargo" "direnv" ];
    buildInputs = with pkgs; [ rustc cargo rustfmt clippy ];
  };

  python-lang = {
    description = "Python project with poetry";
    path = "Code/Python";
    direnv = "use flake";
    requires = [ "poetry" "direnv" ];
    buildInputs = with pkgs; [ python311 poetry ];
  };

  c-lang = {
    description = "C project with gcc and make";
    path = "Code/C";
    direnv = "use flake";
    requires = [ "gcc" "make" ];
    buildInputs = with pkgs; [ gcc make ];
  };

  cpp-lang = {
    description = "C++ project with cmake";
    path = "Code/C++";
    direnv = "use flake";
    requires = [ "cmake" "clang" ];
    buildInputs = with pkgs; [ cmake clang ];
  };

  java-lang = {
    description = "Java project with maven";
    path = "Code/Java";
    direnv = "use flake";
    requires = [ "maven" "java" ];
    buildInputs = with pkgs; [ maven jdk17 ];
  };

  nix-lang = {
    description = "Nix flake project";
    path = "Code/Nix";
    direnv = "use flake";
    requires = [ "nix" ];
    buildInputs = with pkgs; [ nix ];
  };

  r-lang = {
    description = "R project with renv";
    path = "Code/R";
    direnv = "use flake";
    requires = [ "R" "renv" ];
    buildInputs = with pkgs; [ R rPackages.renv ];
  };

  txt-lang = {
    description = "Plain text notes — no flake, no toolchain";
    path = "Notes/txt";
    direnv = "none";
    requires = [ ];
    buildInputs = [ ];
  };

  all-languages = {
    inherit rust-lang python-lang c-lang cpp-lang java-lang nix-lang r-lang txt-lang;
  };

  # ─── Include definitions ─────────────────────────────────────────────────────

  git-include = {
    description = "Initialize git repo and set remote to GitHub";
    provides = [ "git-init" "git-remote" ];
    requires = [ "git" "gh" ];
  };

  overseer-include = {
    description = "Add overseer task runner integration";
    provides = [ "overseer" ];
    requires = [ ];
  };

  all-includes = {
    inherit git-include overseer-include;
  };

  # ─── Nixpkgs instance for flake generation ─────────────────────────────────

  # Use nixos-unstable for all generated flakes
  nixpkgs-for-flakes = import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/tarball/nixos-unstable";
    sha256 = "0000000000000000000000000000000000000000000000000000";
  }) { system = "x86_64-linux"; };

  # ─── Flake generation ────────────────────────────────────────────────────────

  generate-lang-flake = name: lang:
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
            packages.''${system}.default = pkgs.mkShell {
              name = "${lang.description}";
              buildInputs = with pkgs; [ ${bi} ];
            };

            devShells.''${system}.default = self.packages.''${system}.default;
          };
      }
    '';

  generate-lang-setup = name: lang:
    let
      setup-commands =
        if name == "rust" then ''
          mkdir -p "$FORGE_PROJECT_PATH"
          cd "$FORGE_PROJECT_PATH"

          render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

          if [ ! -d .git ]; then
            git init
            git add flake.nix
            git commit -m "init"
          fi

          cat > .envrc << 'ENVEOF'
          use flake
          ENVEOF

          if [ ! -f Cargo.toml ]; then
            nix develop . -c cargo init .
          fi

          direnv allow
        ''
        else if name == "python" then ''
          mkdir -p "$FORGE_PROJECT_PATH"
          cd "$FORGE_PROJECT_PATH"

          render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

          if [ ! -d .git ]; then
            git init
            git add flake.nix
            git commit -m "init"
          fi

          cat > .envrc << 'ENVEOF'
          use flake
          ENVEOF

          if [ ! -f pyproject.toml ]; then
            nix develop . -c poetry init --name "$FORGE_PROJECT_NAME" --quiet
          fi

          direnv allow
        ''
        else ''
          mkdir -p "$FORGE_PROJECT_PATH"
          cd "$FORGE_PROJECT_PATH"

          render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

          if [ ! -d .git ]; then
            git init
            git add flake.nix
            git commit -m "init"
          fi

          cat > .envrc << 'ENVEOF'
          use flake
          ENVEOF

          direnv allow
        '';
    in
    builtins.toFile "setup.sh" ''
      #!/bin/bash
      # forge_description: Scaffold a ${lang.description}
      # forge_requires: ${toString lang.requires}

      set -e

      render_template() {
        local src="$1"
        local dst="$2"
        sed "s/{{PROJECT_NAME}}/$''{FORGE_PROJECT_NAME}/g" "$src" > "$dst"
      }

      if [ "$FORGE_DRY_RUN" = "1" ]; then
        echo "[dry-run] mkdir -p $FORGE_PROJECT_PATH"
        echo "[dry-run] write .envrc"
        echo "[dry-run] write flake.nix"
        echo "[dry-run] nix develop . -c ..."
        echo "[dry-run] direnv allow"
        exit 0
      fi

      ${setup-commands}
    '';

  generate-lang-langwl = name: lang:
    builtins.toFile "lang.wl" ''
      name="${name}"
      desc="${lang.description}"
      path="${lang.path}"
      direnv="${lang.direnv}"
      requires=[${lib.concatMapStringsSep "," (r: "\"${r}\"") lang.requires}]
      setup_priority="10"

      build=""
      run=""
      test=""
      check=""
    '';

  generate-include-includewl = name: inc:
    builtins.toFile "include.wl" (builtins.toJSON {
      inherit (inc) description provides requires;
      version = "1.0";
    });

  generate-include-setup = name: inc:
    let
      script = if name == "git" then ''
        set -e
        cd "$FORGE_PROJECT_PATH"

        if [ ! -d .git ]; then
          git init
        fi

        REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
        git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
      '' else if name == "overseer" then ''
        set -e
        cd "$FORGE_PROJECT_PATH"

        cat > overseer.wl << 'OVSEOF'
        {
          "name": "overseer",
          "builder": "custom",
          "cmd": "${cfg.forge.package}/bin/forge overseer-def $FORGE_PROJECT_NAME",
          "cwd": "$FORGE_PROJECT_PATH"
        }
        OVSEOF
      '' else "";
    in
    builtins.toFile "setup.sh" ''
      #!/bin/bash
      # forge_description: ${inc.description}
      # forge_provides: ${toString inc.provides}
      # forge_requires: ${toString inc.requires}

      set -e

      if [ "$FORGE_DRY_RUN" = "1" ]; then
        echo "[dry-run] ${name} include setup"
        exit 0
      fi

      ${script}
    '';

  # ─── Build generated files ─────────────────────────────────────────────────

  lang-files = lib.foldlAttrs (acc: name: lang: acc // {
    "${name}" = {
      "flake.nix" = generate-lang-flake name lang;
      "setup.sh"  = generate-lang-setup name lang;
      "lang.wl"   = generate-lang-langwl name lang;
    };
  }) {} all-languages;

  include-files = lib.foldlAttrs (acc: name: inc: acc // {
    "${name}" = {
      "include.wl" = generate-include-includewl name inc;
      "setup.sh"  = generate-include-setup name inc;
    };
  }) {} all-includes;

  # The forge package (from the project's nix/package.nix)
  forge-binary = pkgs.callPackage ../nix/package.nix { };

  # ─── Runtime config file ────────────────────────────────────────────────────

  runtime-config = pkgs.writeTextFile {
    name = "forge-runtime-config";
    destination = "/config";
    text = ''
      FORGE_SYNC_BASE="${cfg.syncBase}"
      FORGE_EDITOR="${cfg.editor}"
      FORGE_GITHUB_USER="${cfg.githubUser}"
      FORGE_TMUX_BINARY="tmux"
    '';
  };

in

{
  options.forge = {
    enable = lib.mkEnableOption "forge — tmux sessionizer with includes and overseer integration";

    syncBase = lib.mkOption {
      default = "${config.home.homeDirectory}/sync";
      type = lib.types.path;
      description = "Root directory where all projects live";
      example = "/home/adam/sync";
    };

    editor = lib.mkOption {
      default = "${pkgs.neovim}/bin/nvim";
      type = lib.types.str;
      description = "Editor binary for 'open' and 'edit' commands";
      example = "nvim";
    };

    githubUser = lib.mkOption {
      default = null;
      type = lib.types.nullOr lib.types.str;
      description = "GitHub username used by the git include to set remote URLs";
      example = "AMarek05";
    };

    tmuxBinary = lib.mkOption {
      default = "${pkgs.tmux}/bin/tmux";
      type = lib.types.str;
      description = "Path to the tmux binary";
    };

    languages = lib.mkOption {
      default = [ "rust" "python" "c" "cpp" "nix" "java" "r" "txt" ];
      type = lib.types.listOf lib.types.str;
      description = "List of language packs to ship in the store";
    };

    includes = lib.mkOption {
      default = [ "git" "overseer" ];
      type = lib.types.listOf lib.types.str;
      description = "List of include modules to ship in the store";
    };

    package = lib.mkOption {
      default = forge-binary;
      type = lib.types.package;
      description = "The forge binary package to install";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      cfg.package
    ];

    home.file = {
      "${config.home.homeDirectory}/.local/state/forge".source =
        pkgs.runCommand "forge-state" {
          preferLocalBuild = true;
          allowSubstitutes = false;
        } ''
          mkdir -p $out
          echo '${cfg.syncBase}' > $out/sync_base
          echo '${cfg.editor}' > $out/editor
          ${lib.optionalString (cfg.githubUser != null) ''
            echo '${cfg.githubUser}' > $out/github_user
          ''}
          echo '${cfg.tmuxBinary}' > $out/tmux_binary
        '';

      "${config.home.homeDirectory}/.local/state/forge/languages".source =
        pkgs.runCommand "forge-languages" {
          preferLocalBuild = true;
          allowSubstitutes = false;
        } ''
          mkdir -p $out
          ${lib.concatMapStrings (lang: ''
            lang_name="${lang}"
            lang_dir="$out/$lang_name"
            mkdir -p $lang_dir
            ${lib.concatMapStrings (file: ''
              cp ${lang-files.${lang}."${file}"} $lang_dir/"${file}"
            '') [ "flake.nix" "setup.sh" "lang.wl" ]}
          '') cfg.languages}
        '';

      "${config.home.homeDirectory}/.local/state/forge/includes".source =
        pkgs.runCommand "forge-includes" {
          preferLocalBuild = true;
          allowSubstitutes = false;
        } ''
          mkdir -p $out
          ${lib.concatMapStrings (inc: ''
            inc_name="${inc}"
            inc_dir="$out/$inc_name"
            mkdir -p $inc_dir
            ${lib.concatMapStrings (file: ''
              cp ${include-files.${inc}."${file}"} $inc_dir/"${file}"
            '') [ "include.wl" "setup.sh" ]}
          '') cfg.includes}
        '';
    };

    home.sessionVariables = {
      FORGE_SYNC_BASE = cfg.syncBase;
      FORGE_EDITOR = cfg.editor;
      FORGE_LANG_DIR = "${config.home.homeDirectory}/.local/state/forge/languages";
      FORGE_INCLUDE_DIR = "${config.home.homeDirectory}/.local/state/forge/includes";
      FORGE_GITHUB_USER = cfg.githubUser;
      FORGE_TMUX_BINARY = cfg.tmuxBinary;
    };
  };
}