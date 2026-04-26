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
      "git"
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
      "git"
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
      "git"
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
      "git"
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
      "git"
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
    requires = [ "git" "nix" ];
    buildInputs = [ "nix" ];
  };

  r-lang = {
    description = "R project with renv";
    path = "Code/R";
    direnv = "use flake";
    requires = [
      "git"
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
    requires = [ "git" ];
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
            TEMPLATE_DIR="$HOME/.local/share/nvim/site/lua/overseer/template/forge"
            mkdir -p "$TEMPLATE_DIR"
            parse_wl_field() {
              grep -E "^${1}=" "$FORGE_PROJECT_PATH/.wl" 2>/dev/null | head -1 | sed 's/^[^=]*=//' | tr -d '"' | xargs
            }
            BUILD_CMD=$(parse_wl_field "build")
            RUN_CMD=$(parse_wl_field "run")
            TEST_CMD=$(parse_wl_field "test")
            CHECK_CMD=$(parse_wl_field "check")
            BUILD_CMD="${BUILD_CMD:-nix build}"
            RUN_CMD="${RUN_CMD:-nix run}"
            TEST_CMD="${TEST_CMD:-nix flake check}"
            CHECK_CMD="${CHECK_CMD:-nix flake check}"
            escape_lua() { printf '%s' "$1" | sed 's/["\\]/\\&/g'; }
            write_template() {
              local task_name="$1" cmd="$2" tag="$3"
              local escaped=$(escape_lua "$cmd")
              cat > "${TEMPLATE_DIR}/${FORGE_PROJECT_NAME}_${task_name}.lua" << LUAEOF
return {
  name = "${FORGE_PROJECT_NAME}:${task_name}",
  builder = function()
    return {
      cmd = { "bash", "-c", "${escaped}" },
      cwd = "${FORGE_PROJECT_PATH}",
      components = { "default" },
    }
  end,
  tags = { overseer.TAG.${tag} },
  desc = "${FORGE_PROJECT_NAME}:${task_name}",
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

  zsh-completion = builtins.toFile "_forge" ''
#compdef forge

forge() {
  local -a commands
  commands=(
    "create:Create a new project"
    "remove:Remove a project from the index"
    "list:List all projects"
    "sync:Re-scan FORGE_SYNC_BASE and rebuild the index"
    "cd:Print project path to stdout"
    "session:Switch to or create a tmux session"
    "pick:Interactive fzf session picker"
    "setup:Run setup scripts for a project"
    "include:List or show include modules"
    "lang:List or add language packs"
    "overseer:Run or manage overseer.nvim task templates"
    "overseer-def:Print JSON overseer task definition"
    "edit:Edit project's .wl in \$EDITOR"
    "open:Open project directory in \$EDITOR"
  )

  if [[ "${words[CURRENT]}" == -* ]]; then
    _describe "options" "(
      --help 'Show help'
      --version 'Show version'
    )
    return
  fi

  if (( CURRENT == 2 )); then
    _describe "commands" commands
    return
  fi

  local cmd="${words[2]}"
  case "$cmd" in
    create)
      _describe "flags" "(
        --lang 'Language (required)'
        --no-open 'Skip opening .wl in \$EDITOR'
        --setup 'Run setup scripts after creating .wl'
        --include 'Pre-populate includes field (comma-separated)'
        --path 'Override project path'
        --run 'Run arbitrary shell command after creation'
        --editor 'Open \$EDITOR after full creation'
        --dry-run 'Print actions without executing'
      )"
      ;;
    remove|list|cd|edit|open|overseer-def)
      _message "no additional args"
      ;;
    session)
      _describe "flags" "(
        --setup 'Run setup scripts in the session'
        --open 'Open project in \$EDITOR after switching'
      )"
      ;;
    pick)
      _describe "flags" "(
        --tags 'Filter by tags (comma-separated)'
      )"
      ;;
    setup)
      _describe "flags" "(
        --dry-run 'Print actions without executing'
      )"
      ;;
    include)
      _describe "flags" "(
        --list 'List all available includes'
      )"
      ;;
    lang)
      _describe "flags" "(
        --list 'List all available languages'
        --add 'Add a new language pack'
      )"
      ;;
    overseer)
      _describe "flags" "(
        --regen 'Regenerate all project templates'
        --rm 'Remove project's templates'
        --setup 'Run setup scripts for overseer include'
      )"
      ;;
  esac
}

forge "$@"
'';

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

    home.file."share/zsh/site-functions/_forge".text = zsh-completion;

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

