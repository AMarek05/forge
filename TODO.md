# forge — tmux sessionizer with includes and overseer integration

**Fully buildable with Nix + deployable as a Home-manager module.**

---

## Current Status (2026-04-29)

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

~/.forge/                      # writable runtime state
  ├── index.json               # project index (v3, structural fields only)
  └── config                   # runtime config

~/.local/share/zsh/site-functions/_forge  # zsh completion (installed by HM)
~/.local/share/nvim/site/lua/overseer/template/forge/  # overseer templates (per project)
  ├── myproject.lua
  └── ...

<project>/.forge/              # per-project state (moves with project dir)
  ├── state                    # all .wl field values (name/lang/desc/tags/includes/build/run/test/check/last_wl_mtime)
  └── applied-includes         # includes whose setup.sh has already run
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

### Comprehensive test suite — IN PROGRESS

Added `src/lib.rs` (forge library crate exposing all modules for testing).
Added `src/wl_parser.rs` — `#[cfg(test)] mod tests` with full unit coverage for:
  - `strip_quotes`: double/single quotes, no quotes, whitespace
  - `parse_json_array`: empty, single, multi, whitespace, not-array, single-quotes
  - `parse_wl`: minimal, all-fields, empty-arrays, comments, malformed-line errors, unclosed-bracket, unquoted-string, duplicate-key-last-wins
  - `parse_lang_wl`: basic, optional-fields-missing
Added `tests/integration/main.rs` — Rust binary that wraps `tests/run.sh all`

**To verify (Adam runs):**
```
cd ~/projects/sync-launcher
cargo test --lib                           # unit tests (wl_parser, applied_includes, project_state)
cargo test --test integration              # integration suite via tests/run.sh
bash tests/run.sh all                     # bash test runner (shell + integration)
```

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
- [x] `includes/overseer/include.wl` — `provides=["overseer"]`
- [x] `src/commands/overseer.rs` — all subcommands implemented
- [x] `forge overseer-def <name>` — hooked up to CLI

### ZSH Completion ✅
- [x] Hand-written `module/completion.zsh` with project name autocompletion
- [x] `@JQ@` placeholder replaced at HM eval time via `builtins.replaceStrings`
- [x] `--help` flag on all subcommands
- [x] `programs.zsh.initContent` with `lib.mkOrder 550` for fpath setup

### Field prefill & validation
- [x] `forge check [<project>]` — validate `.wl` syntax and field integrity
  - Syntax: malformed lines, unclosed brackets, bad array/json structure
  - `lang` field: resolves against known languages (from HM config)
  - `includes` field: each entry resolves against available includes
  - `tags` field: validates as string array
  - Warns if `build`/`run`/`test`/`check` are empty (overseer will use `nix build` fallback)
  - Fails with exit code non-zero on any error
  - `forge check` (no arg) runs on index state file and all known projects
  - `forge check <name>` runs on single project

### Field prefill ✅
- [x] `forge create <name> --lang rust --include git` — generates `.wl` with all fields pre-declared:
    - `name=<name>` from arg; `lang=rust`, `includes=[git]` from flags
    - `desc=""`, `tags=[]` — empty, user fills
    - `build`, `run`, `test`, `check` — populated from `lang.wl` defaults
    - `overseer_template` if applicable
    - All fields present, none missing
  - Opens editor on the `.wl` (unless `--no-open`), auto-syncs on close
  - verify_and_diff on close: syntax check + include diff + field diff + index update + state save

### Applied-includes tracking (done ✅)
- [x] Per-project `.forge/applied-includes` tracks which includes have had their setup.sh run
- [x] `forge sync`: diffs includes vs applied-includes, runs missing setups, updates applied-includes
- [x] `forge edit <project>`: opens `.wl`, verify_and_diff on close (syntax check + include diff + field diff + index update)
- [x] `forge create`: saves applied-includes after setup, re-diffs after editor close
- [x] `forge pick` ctrl-e: same verify_and_diff flow
- [x] `forge sync`: warns when removing stale entries (`.wl` gone), warns on parse failure
- [x] Index: `~/.forge-index.json` → `~/.forge/index.json` with auto-migration from old location
- [x] Per-project `.forge/state`: tracks all .wl fields, written after every verified edit

### Healthcheck ✅
- [x] `forge health` — general system state validator
  - Index file (`~/.forge/index.json`): valid JSON, non-empty projects array
  - Each project `.wl`: parseable, no missing required fields
  - Each project `.forge/state`: present and readable
  - Detects and flags: projects with no `name`, duplicate `name` entries, stale `path` pointing to missing directory
  - `--fix` flag to auto-correct: remove stale entries and duplicate name entries, saves updated index
  - Exit code: 0 if healthy, non-zero if issues found
  - Output format:
    ```
    ✅ index.json: valid
    ✅ "myproject": .wl valid
    ⚠️  project "dup-name": duplicate name (appears in both ...)
    ❌ project "broken": .wl syntax error — ...
    ```

### pick ctrl-e/ctrl-o ✅
- [x] ctrl-e: opens `.wl` directly in `$EDITOR`
- [x] ctrl-o: chdirs to project, opens project dir in Oil (`nvim -c Oil .`)

### End-to-end testing — IN PROGRESS

Test suite infrastructure added this session:
- `src/lib.rs` — forge library crate exposing all modules
- `src/wl_parser.rs` — 20+ unit tests covering parse functions
- `tests/integration/main.rs` — Rust wrapper binary for test runner
- `tests/unit/queries.md` — 11 Rust unit test query specs (already in repo)

Adam, run these to validate:
```
# Unit tests (Rust)
cd ~/projects/sync-launcher
cargo test --lib

# Full integration suite
bash tests/run.sh all

# Or cargo test for integration (binary wrapper)
cargo test --test integration
```

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
- [ ] Test `forge edit <name>` — change name, verify index updated
- [ ] Test `forge health --fix` — verify it removes stale/duplicate entries