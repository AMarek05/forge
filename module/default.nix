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

  # FIX: Avoid naming the module argument `forge` to prevent collisions with `config.forge`.
  # Pass your package via `extraSpecialArgs = { forgePkg = ...; }` in your flake setup.
  forge-binary = args.forgePkg or null;

  # ─── Language definitions ───────────────────────────────────────────────────

  rust-lang = {
    description = "Rust project with cargo";
    path = "Code/Rust";
    direnv = "use flake";
    requires = [
      "cargo"
      "direnv"
    ];
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
    requires = [
      "poetry"
      "direnv"
    ];
    buildInputs = [
      "python311"
      "poetry"
    ];
  };

  c-lang = {
    description = "C project with gcc and make";
    path = "Code/C";
    direnv = "use flake";
    requires = [
      "gcc"
      "make"
    ];
    buildInputs = [
      "gcc"
      "make"
    ];
  };

  cpp-lang = {
    description = "C++ project with cmake";
    path = "Code/C++";
    direnv = "use flake";
    requires = [
      "cmake"
      "clang"
    ];
    buildInputs = [
      "cmake"
      "clang"
    ];
  };

  java-lang = {
    description = "Java project with maven";
    path = "Code/Java";
    direnv = "use flake";
    requires = [
      "maven"
      "java"
    ];
    buildInputs = [
      "maven"
      "jdk17"
    ];
  };

  nix-lang = {
    description = "Nix flake project";
    path = "Code/Nix";
    direnv = "use flake";
    requires = [ "nix" ];
    buildInputs = [ "nix" ];
  };

  r-lang = {
    description = "R project with renv";
    path = "Code/R";
    direnv = "use flake";
    requires = [
      "R"
      "renv"
    ];
    buildInputs = [
      "R"
      "renv"
    ];
  };

  txt-lang = {
    description = "Plain text notes — no flake, no toolchain";
    path = "Notes/txt";
    direnv = "none";
    requires = [ ];
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
    requires = [
      "git"
      "gh"
    ];
  };

  overseer-include = {
    description = "Add overseer task runner integration";
    provides = [ "overseer" ];
    requires = [ ];
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
            packages.''${system}.default = pkgs.mkShell {
              name = "${lang.description}";
              buildInputs = with pkgs; [ ${bi} ];
            };
            devShells.''${system}.default = self.packages.''${system}.default;
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
            render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix
            if [ ! -d .git ]; then git init; git add flake.nix; git commit -m "init"; fi
            cat > .envrc << 'ENVEOF'
            use flake
            ENVEOF
            if [ ! -f Cargo.toml ]; then nix develop . -c cargo init .; fi
            direnv allow
          ''
        else if name == "python" then
          ''
            mkdir -p "$FORGE_PROJECT_PATH"
            cd "$FORGE_PROJECT_PATH"
            render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix
            if [ ! -d .git ]; then git init; git add flake.nix; git commit -m "init"; fi
            cat > .envrc << 'ENVEOF'
            use flake
            ENVEOF
            if [ ! -f pyproject.toml ]; then nix develop . -c poetry init --name "$FORGE_PROJECT_NAME" --quiet; fi
            direnv allow
          ''
        else
          ''
            mkdir -p "$FORGE_PROJECT_PATH"
            cd "$FORGE_PROJECT_PATH"
            render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix
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
      # FIX: In Nix multiline strings, standard bash variables don't need escaping.
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
      requires=[${lib.concatMapStringsSep "," (r: "\"${r}\"") lang.requires}]
      setup_priority="10"
      build=""
      run=""
      test=""
      check=""
    '';

  generate-include-includewl =
    name: inc:
    builtins.toFile "include.wl" (
      builtins.toJSON {
        inherit (inc) description provides requires;
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
            mkdir -p "$FORGE_PROJECT_PATH/.forge/"
            cat > "$FORGE_PROJECT_PATH/.forge/overseer.lua" << 'OVSEOF'
            return {
              default_task = "build",
              tasks = {
                build = { command = "cargo build", name = "build", run-type = "on_save", trigger = "*.rs" },
                run   = { command = "cargo run",   name = "run",   run-type = "on_save", trigger = "*.rs" },
                test  = { command = "cargo test",  name = "test",  run-type = "on_save", trigger = "*.rs" },
              },
            }
            OVSEOF
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

  # FIX: mapAttrs is significantly cleaner and avoids potential strict evaluation issues
  lang-files = builtins.mapAttrs (name: lang: {
    "flake.nix" = generate-lang-flake name lang;
    "setup.sh" = generate-lang-setup name lang;
    "lang.wl" = generate-lang-langwl name lang;
  }) all-languages;

  include-files = builtins.mapAttrs (name: inc: {
    "include.wl" = generate-include-includewl name inc;
    "setup.sh" = generate-include-setup name inc;
  }) all-includes;

in

{
  options.forge = {
    enable = lib.mkEnableOption "forge — tmux sessionizer";

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

  # FIX: Removed the invalid `let` assignment from inside the attrset
  config = lib.mkIf cfg.enable {

    home.packages = lib.mkIf (cfg.package != null) [ cfg.package ];

    home.sessionVariables = {
      FORGE_SYNC_BASE = cfg.syncBase;
      FORGE_EDITOR = cfg.editor;
      FORGE_GITHUB_USER = cfg.githubUser;
      FORGE_TMUX_BINARY = cfg.tmuxBinary;

      FORGE_LANG_DIR =
        pkgs.runCommand "forge-languages"
          {
            preferLocalBuild = true;
            allowSubstitutes = false;
          }
          ''
            mkdir -p $out
            ${lib.concatMapStrings (lang: ''
              mkdir -p $out/${lang}
              cp ${lang-files.${lang}."flake.nix"} $out/${lang}/
              cp ${lang-files.${lang}."setup.sh"}  $out/${lang}/
              cp ${lang-files.${lang}."lang.wl"}   $out/${lang}/
            '') cfg.languages}
          '';

      FORGE_INCLUDE_DIR =
        pkgs.runCommand "forge-includes"
          {
            preferLocalBuild = true;
            allowSubstitutes = false;
          }
          ''
            mkdir -p $out
            ${lib.concatMapStrings (inc: ''
              mkdir -p $out/${inc}
              cp ${include-files.${inc}."include.wl"} $out/${inc}/
              cp ${include-files.${inc}."setup.sh"}  $out/${inc}/
            '') cfg.includes}
          '';
    };
  };
}

