# forge

tmux sessionizer backed by `~/sync` with plugin-style includes and overseer.nvim integration.

**Fully buildable with Nix + deployable as a Home-manager module.**

---

## Architecture

### Single source of truth: Nix store

All language packs and includes live in the Nix store as immutable files generated at module evaluation time. No mutable `~/.forge/` directory for shipped content — only runtime state (index, config) is in `~/.local/state/forge/`.

```
store: /nix/store/<hash>-forge-0.1.0/
  └── share/forge/
      ├── languages/         # generated at module eval time
      │   ├── rust/flake.nix  # programmatically generated per language
      │   ├── rust/setup.sh
      │   ├── python/
      │   └── ...
      └── includes/          # generated at module eval time
          ├── git/setup.sh
          └── overseer/setup.sh

~/.local/state/forge/         # writable runtime state only
  ├── index.json              # project index
  └── config                  # runtime config (FORGE_SYNC_BASE, editor, etc.)
```

### Home-manager module

The `forge` home-manager module generates language flakes and includes at **module evaluation time**, shipping them to the store. The binary references them via `FORGE_DIR/share/forge/`.

```nix
# In home-manager configuration
forge = {
  enable = true;
  syncBase = "${config.home.homeDirectory}/sync";
  githubUser = "AMarek05";
  editor = "${pkgs.neovim}/bin/nvim";
  languages = ["rust" "python" "c" "cpp" "nix" "java" "r"];
  includes = ["git" "overseer"];
};
```

---

## Language generation (in module)

Each language's `flake.nix` is generated programmatically at module eval time:

```nix
languages = {
  rust = {
    description = "Rust project with cargo";
    path = "Code/rust";
    buildInputs = with pkgs; [ rustc cargo rustfmt clippy ];
  };
  python = {
    description = "Python project with poetry";
    path = "Code/python";
    buildInputs = with pkgs; [ python311 poetry ];
  };
  # ... etc
};

generate-flake = lang: pkgs: ''
  {
    description = "${lang.description}";
    inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    outputs = { self, nixpkgs }:
      let
        system = "x86_64-linux";
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.${system}.default = pkgs.mkShell {
          name = "${lang.description}";
          buildInputs = with pkgs; ${toString lang.buildInputs};
        };
        devShells.${system}.default = self.packages.${system}.default;
      };
  }
'';
```

---

## Include generation (in module)

Includes are bash scripts generated at module eval time. Runtime variables (e.g., `$FORGE_GITHUB_USER`) are stored as literal strings in the script — substituted at runtime by the binary.

```nix
includes = {
  git = {
    description = "Initialize git repo and set remote to GitHub";
    provides = [ "git-init" "git-remote" ];
    requires = [ "git" "gh" ];
    setupScript = ''
      set -e
      cd "$FORGE_PROJECT_PATH"
      if [ ! -d .git ]; then
        git init
      fi
      REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
      git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
    '';
  };
  overseer = {
    description = "Add overseer task runner integration";
    provides = [ "overseer" ];
    requires = [ ];
    setupScript = ''...'';
  };
};
```

---

## Directory layout (project source)

```
forge/
├── Cargo.toml
├── flake.nix              # nix build + dev shell
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── commands/
│   │   ├── create.rs
│   │   ├── remove.rs
│   │   ├── list.rs
│   │   ├── sync.rs
│   │   ├── cd.rs
│   │   ├── session.rs
│   │   ├── pick.rs
│   │   ├── setup.rs
│   │   ├── lang.rs
│   │   ├── include.rs
│   │   └── overseer.rs
│   ├── index.rs
│   ├── wl_parser.rs
│   ├── tmux.rs
│   ├── include.rs
│   ├── language.rs
│   └── config.rs
├── languages/             # default language packs (for nix build development)
│   ├── rust/setup.sh, flake.nix.template, lang.wl
│   └── ...
├── includes/              # default includes (for nix build development)
│   ├── git/, overseer/
├── completions/
│   └── zsh/
├── module/
│   └── default.nix        # home-manager module (generated languages + includes)
├── nix/
│   └── package.nix
└── TODO.md
```

---

## Module options

```nix
options.forge = {
  enable = lib.mkEnableOption "forge tmux sessionizer";
  syncBase = lib.mkOption {
    default = "${config.home.homeDirectory}/sync";
    type = lib.types.path;
    description = "Where projects live";
  };
  editor = lib.mkOption {
    default = "${pkgs.neovim}/bin/nvim";
    type = lib.types.str;
    description = "Editor binary for 'open' and 'edit' commands";
  };
  githubUser = lib.mkOption {
    default = null;
    type = lib.types.nullOr lib.types.str;
    description = "GitHub username for git include remote setup";
  };
  languages = lib.mkOption {
    default = ["rust" "python" "c" "cpp" "nix" "java" "r"];
    type = lib.types.listOf lib.types.str;
    description = "Which language packs to include";
  };
  includes = lib.mkOption {
    default = ["git" "overseer"];
    type = lib.types.listOf lib.types.str;
    description = "Which include modules to include";
  };
};
```

---

## Nix build (development)

```nix
# flake.nix
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
          buildInputs = with pkgs; [ rustc cargo rust-analyzer rustfmt clippy ];
        };
      }
    );
}
```

---

## TODOs

### Phase 1 ✅
- [x] Binary builds with `nix build`
- [x] All 8 language packs created (c, cpp, java, nix, python, r, rust, txt)
- [x] Includes: git, overseer
- [x] `cargo check` passes

### Phase 2 ✅
- [x] `forge cd` emits `cd <path>` for evalability, `--print` for bare path
- [x] `forge open` chdirs before spawning `$EDITOR`, auto-detects nvim + appends `-c Oil`
- [x] `forge pick` silences tmux stderr, ctrl-o/ctrl-e chdir before nvim
- [x] `forge session --open` flag added
- [x] All language flake templates use hardcoded x86_64-linux (no complex SYSTEM var)
- [x] All language setup scripts git init + add flake.nix before nix develop

### Home-manager module
- [ ] Write `module/default.nix` with:
  - [ ] All language definitions (name, description, path, buildInputs)
  - [ ] All include definitions (description, provides, requires, setupScript)
  - [ ] `lib.cleanSource` of `./languages/` and `./includes/` for dev
  - [ ] Generate `flake.nix` per language via `builtins.toFile`
  - [ ] Generate `setup.sh` per language via `builtins.toFile`
  - [ ] Generate `include.wl` per include via `builtins.toJSON`
  - [ ] Generate `setup.sh` per include via `builtins.toFile`
  - [ ] Copy generated files to `$out/share/forge/languages/` and `$out/share/forge/includes/`
  - [ ] Write wrapper script to `$out/bin/forge` that sets `FORGE_DIR`
  - [ ] Options: enable, syncBase, editor, githubUser, languages, includes
  - [ ] Config: write runtime config to `~/.local/state/forge/config`
  - [ ] Install binary to profile
- [ ] Test module with `home-manager switch`
- [ ] Remove mutable `~/.forge/config.sh` approach from binary (use `$FORGE_DIR/share/forge/` for langs/includes + env vars for runtime config)

### Binary runtime refactor
- [ ] Binary reads `FORGE_LANG_DIR` and `FORGE_INCLUDE_DIR` from env (set by wrapper)
- [ ] Binary reads `FORGE_SYNC_BASE` from env or `~/.local/state/forge/config`
- [ ] Remove `~/.forge/config.sh` sourcing from setup scripts
- [ ] Update `create.rs` to use env-based paths
- [ ] Index goes to `~/.local/state/forge/index.json`

### Remaining fixes
- [ ] Test `forge create x --lang rust --no-open` end-to-end (cargo init works)
- [ ] Test `forge pick` ctrl-o opens nvim with Oil
- [ ] Test `forge session x --open` opens nvim with Oil
- [ ] Test `forge create x --lang python --no-open` (poetry init via nix develop)
- [ ] ZSH completions (`completions/zsh/_forge`)