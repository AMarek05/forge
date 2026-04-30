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

  # ─── Language definitions ───────────────────────────────────────────────────

  rust-lang = {
    description = "Rust project with cargo";
    path = "Code/Rust";
    direnv = "use flake";
    buildInputs = [
      "rustc"
      "cargo"
      "rustfmt"
      "clippy"
    ];
  };

  python-lang = {
    description = "Python project with poetry";
    path = "Code/Python";
    direnv = "use flake";
    buildInputs = [
      "python311"
      "poetry"
    ];
  };

  c-lang = {
    description = "C project with gcc and make";
    path = "Code/C";
    direnv = "use flake";
    buildInputs = [
      "gcc"
      "make"
    ];
  };

  cpp-lang = {
    description = "C++ project with cmake";
    path = "Code/C++";
    direnv = "use flake";
    buildInputs = [
      "cmake"
      "clang"
    ];
  };

  java-lang = {
    description = "Java project with maven";
    path = "Code/Java";
    direnv = "use flake";
    buildInputs = [
      "maven"
      "jdk17"
    ];
  };

  nix-lang = {
    description = "Nix flake project";
    path = "Code/Nix";
    direnv = "use flake";
    buildInputs = [ "nix" ];
  };

  r-lang = {
    description = "R project with renv";
    path = "Code/R";
    direnv = "use flake";
    buildInputs = [
      "R"
      "renv"
    ];
  };

  txt-lang = {
    description = "Plain text notes — no flake, no toolchain";
    path = "Notes/txt";
    direnv = "none";
    buildInputs = [ ];
  };

  all-languages = {
    rust = rust-lang;
    python = python-lang;
    c = c-lang;
    cpp = cpp-lang;
    java = java-lang;
    nix = nix-lang;
    r = r-lang;
    txt = txt-lang;
  };

  # ─── Include definitions ─────────────────────────────────────────────────────

  git-include = {
    description = "Initialize git repo and set remote to GitHub";
    provides = [
      "git-init"
      "git-remote"
    ];
  };

  overseer-include = {
    description = "Add overseer task runner integration";
    provides = [ "overseer" ];
  };

  all-includes = {
    git = git-include;
    overseer = overseer-include;
  };

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
    let
      script =
        if name == "git" then
          ''
            set -e
            cd "$FORGE_PROJECT_PATH"
            if [ ! -d .git ]; then git init; fi
            REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
            git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
          ''
        else if name == "overseer" then
          ''
            set -e
            TEMPLATE_DIR="$HOME/.local/share/nvim/site/lua/overseer/template/forge"
            mkdir -p "$TEMPLATE_DIR"
            parse_wl_field() {
              grep -E "^$1=" "$FORGE_PROJECT_PATH/.wl" 2>/dev/null | head -1 | sed 's/^[^=]*=//' | tr -d '"' | xargs
            }
            BUILD_CMD=$(parse_wl_field "build")
            RUN_CMD=$(parse_wl_field "run")
            TEST_CMD=$(parse_wl_field "test")
            CHECK_CMD=$(parse_wl_field "check")

            BUILD_CMD="''${BUILD_CMD:-nix build}"
            RUN_CMD="''${RUN_CMD:-nix run}"
            TEST_CMD="''${TEST_CMD:-nix flake check}"
            CHECK_CMD="''${CHECK_CMD:-nix flake check}"

            escape_lua() { printf '%s' "$1" | sed 's/["\\]/\\&/g'; }
            write_template() {
              local task_name="$1" cmd="$2" tag="$3"
              local escaped=$(escape_lua "$cmd")
              cat > "''${TEMPLATE_DIR}/''${FORGE_PROJECT_NAME}_''${task_name}.lua" << LUAEOF
            return {
              name = "''${FORGE_PROJECT_NAME}:''${task_name}",
              builder = function()
                return {
                  cmd = { "bash", "-c", "''${escaped}" },
                  cwd = "''${FORGE_PROJECT_PATH}",
                  components = { "default" },
                }
              end,
              tags = { overseer.TAG.''${tag} },
              desc = "''${FORGE_PROJECT_NAME}:''${task_name}",
            }
            LUAEOF
            }
            write_template "build" "$BUILD_CMD" "BUILD"
            write_template "run" "$RUN_CMD" "RUN"
            write_template "check" "$CHECK_CMD" "TEST"
          ''
        else
          "";
    in
    builtins.toFile "setup.sh" ''
      #!/bin/bash
      set -e
      if [ "$FORGE_DRY_RUN" = "1" ]; then echo "[dry-run] ${name} include"; exit 0; fi
      ${script}
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

  # ─── Config file generation ──────────────────────────────────────────────────

  # Hardcoded default languages — always included from repo
  default-languages = lib.attrNames all-languages;
  default-includes  = lib.attrNames all-includes;

  # ─── Lang dir: default/ (HM-managed store) + custom/ (HM-managed, user-editable) ───
  lang-dir  = pkgs.runCommand "forge-languages" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    mkdir -p $out/default $out/custom
    ${lib.concatMapStrings (lang: ''
      mkdir -p $out/default/${lang}
      cp ${lang-files.${lang}.flake_nix} $out/default/${lang}/flake.nix
      cp ${lang-files.${lang}.setup_sh}  $out/default/${lang}/setup.sh
      cp ${lang-files.${lang}.lang_wl}   $out/default/${lang}/lang.wl
    '') default-languages}
  '';

  include-dir = pkgs.runCommand "forge-includes" {
    preferLocalBuild = true;
    allowSubstitutes = false;
  } ''
    mkdir -p $out/default $out/custom
    ${lib.concatMapStrings (inc: ''
      mkdir -p $out/default/${inc}
      cp ${include-files.${inc}.include_wl} $out/default/${inc}/include.wl
      cp ${include-files.${inc}.setup_sh}   $out/default/${inc}/setup.sh
    '') default-includes}
  '';

  # ─── Langs catalog JSON ───────────────────────────────────────────────────────
  # Written to ~/.forge/langs.json — consumed by `forge sync --langs`
  langs-catalog-json = builtins.toJSON (
    lib.mapAttrsToList (name: lang: {
      inherit name;
      description = lang.description;
      lang_wl = {
        name     = name;
        desc     = lang.description;
        path     = lang.path;
        direnv   = lang.direnv;
        build    = "";
        run      = "";
        test     = "";
        check    = "";
      };
    }) all-languages
  );

  # ─── Includes catalog JSON ───────────────────────────────────────────────────
  # Written to ~/.forge/includes.json — consumed by `forge sync --includes`
  includes-catalog-json = builtins.toJSON (
    lib.mapAttrsToList (name: inc: {
      inherit name;
      description = inc.description;
      provides    = inc.provides;
      setup_sh    = builtins.readFile (generate-include-setup name inc);
    }) all-includes
  );

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

    languages = lib.mkOption {
      default = [
        "rust"
        "python"
        "c"
        "cpp"
        "nix"
        "java"
        "r"
        "txt"
      ];
      type = lib.types.listOf lib.types.str;
      description = "Language packs to generate";
    };

    includes = lib.mkOption {
      default = [
        "git"
        "overseer"
      ];
      type = lib.types.listOf lib.types.str;
      description = "Include modules to generate";
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

    # Set env var at activation time via wrapper script that forges calls
    home.file."${cfg.configDir}/forge.env" = {
      text = "FORGE_CONFIG_DIR=${cfg.configDir}";
    };

    home.sessionVariables = {
      FORGE_CONFIG_DIR = cfg.configDir;
    };
  };
}