# forge — tmux sessionizer with includes and overseer integration

**Fully buildable with Nix + deployable as a Home-manager module.**

---

## Current Status (2026-04-27)

### What's done

**Binary**: builds via `nix build`. All commands work (`create`, `cd`, `open`, `pick`, `session`, `lang`, `include`).

**Module** (`module/default.nix`): generates language flakes, `setup.sh`, and `lang.wl` for all 8 languages at module eval time — shipped to Nix store. Same for includes (git, overseer). Exports `homeManagerModules` via flake output.

**Shell completion**: hand-written zsh completion in `module/completion.zsh`. `@JQ@` placeholder replaced at HM eval time with `${pkgs.jq}/bin/jq`. Project names autocompleted for `remove`, `list`, `cd`, `edit`, `open`, `overseer-def`. All subcommands have `--help` flag.

**Key commits (pushed to `AMarek05/forge.git` main)**:
- `dc8c1b5` — fix: rewrite completion with exact CLI flags for each command
- `ed6e528` — fix: use initContent with lib.mkOrder instead of deprecated initExtraBeforeCompInit
- `ee435cf` — add --help flag to all command completions
- `5f94b86` — fix: only show project names at position 3 (not on subsequent tabs)
- `beb292f` — rename completion.nix → completion.zsh, fix function name, fix array syntax

---

## Architecture

### Single source of truth: Nix store

All language packs and includes live in the Nix store as immutable files generated at module evaluation time. No mutable `~/.forge/` directory for shipped content — only runtime state (index, config) lives in `~/.local/state/forge/`.

```
store: /nix/store/<hash>-forge-0.1.0/
  └── share/forge/
      ├── languages/
      │   ├── rust/flake.nix, setup.sh, lang.wl
      │   └── ...
      └── includes/
          ├── git/setup.sh
          └── overseer/setup.sh

~/.local/state/forge/         # writable runtime state only
  ├── index.json              # project index
  └── config                  # runtime config

~/.local/share/zsh/site-functions/_forge  # zsh completion (installed by HM)
~/.local/share/nvim/site/lua/overseer/template/forge/  # overseer templates (per project)
  ├── myproject.lua
  └── ...
```

### Home-manager module

```nix
forge = {
  enable = true;
  syncBase = "${config.home.homeDirectory}/sync";
  githubUser = "AMarek05";
  editor = "${pkgs.neovim}/bin/nvim";
  languages = ["rust" "python" "c" "cpp" "nix" "java" "r" "txt"];
  includes = ["git" "overseer"];
};
```

---

## Overseer Integration

### How overseer.nvim works

**Templates** are `.lua` files placed under `lua/overseer/template/<path>/` in the nvim config or any directory in `template_dirs`. They return:
```lua
return {
  name = "Task Name",
  builder = function(params)
    return {
      cmd = { "nix", "build" },
      cwd = "/path/to/project",
      components = { "default" },
    }
  end,
  condition = { dir = "/path/to/project" },  -- optional
  tags = { overseer.TAG.BUILD },              -- optional
  desc = "Optional description",
}
```

**Template location:** `~/.local/share/nvim/site/lua/overseer/template/forge/`

**Template regeneration:** `forge overseer --regen` iterates all projects in `index.json`, reads each `.wl`, and writes per-project `.lua` templates.

**forge overseer command:**
- `forge overseer` — open overseer picker
- `forge overseer --regen` — regenerate all project templates
- `forge overseer <name>` — regenerate single project template
- `forge overseer --rm <name>` — remove project template

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
- [x] All language setup scripts: render flake template → git init → git add flake.nix → git commit → .envrc → lang init → direnv allow

### Phase 3 — Home-manager module ✅
- [x] `module/default.nix` with all language definitions
- [x] All include definitions
- [x] All flake/setup/langwl generators
- [x] `homeManagerModules.default` exported from flake.nix
- [x] Commits pushed to `AMarek05/forge.git` main
- [x] Adam's sys flake updated and switched

### Overseer integration ✅
- [x] `includes/overseer/setup.sh` — writes per-project Lua templates
- [x] `includes/overseer/include.wl` — `provides=["overseer"]`, `requires=[]`
- [x] `src/commands/overseer.rs` — all subcommands implemented
- [x] `forge overseer-def <name>` — hooked up to CLI

### ZSH Completion ✅
- [x] Hand-written `module/completion.zsh` with project name autocompletion
- [x] `@JQ@` placeholder replaced at HM eval time via `builtins.replaceStrings`
- [x] `--help` flag on all subcommands
- [x] `programs.zsh.initContent` with `lib.mkOrder 550` for fpath setup

### End-to-end testing — PENDING
Requires Adam's environment to verify.

- [ ] `forge create x --lang rust --no-open` — cargo init, git commit, direnv
- [ ] `forge create x --lang python --no-open` — poetry init via nix develop
- [ ] `forge create x --lang c --no-open` — generic lang
- [ ] `forge create x --lang rust --include git` — verify git remote set
- [ ] `forge list` — shows all created projects
- [ ] `forge cd x --print` — returns correct path
- [ ] `forge overseer --regen` — generates Lua templates
- [ ] `:OverseerRun` in nvim — shows forge tasks for a project
- [ ] Test `forge pick` ctrl-o opens nvim with Oil
- [ ] Test `forge session x --open` opens nvim with Oil

---

## Test suite

Run with: `cd tests && ./run.sh` (or `./run.sh <target>`)

```
tests/
├── run.sh                    # test runner (all|unit|module|integration|shell)
├── unit/queries.md           # Rust unit test queries (wl_parser, index, config)
├── module/queries.md         # Nix module eval test queries
├── integration/suite.sh      # end-to-end forge workflow tests
└── shell/completion-tests.sh # completion generation + --help/--version smoke
```