# forge — tmux sessionizer with includes and overseer integration

**Fully buildable with Nix + deployable as a Home-manager module.**

---

## Current Status (2026-04-26)

### What's done

**Binary**: builds via `nix build`. All commands work (`create`, `cd`, `open`, `pick`, `session`, `lang`, `include`).

**Module** (`module/default.nix`): generates language flakes, `setup.sh`, and `lang.wl` for all 8 languages at module eval time — shipped to Nix store. Same for includes (git, overseer). Exports `homeManagerModules` via flake output.

**Key commits (pushed to `AMarek05/forge.git` main)**:
- `36f06c9` — Fix: pass lib as arg to module import not let binding
- `798a682` — Fix lib.homeManagerModules: pass lib explicitly
- `4a9a1af` — Add lib.homeManagerModules export and inputs default
- `ad9dbcd` — Fix inclusion of package, remove dangerous pkgs.forge (Adam's)
- `8d73254` — module: add git to all language requires

**Adam's sys flake** (`~/sys/modules/forge.nix`): updated to use `inputs.forge.homeManagerModules.${pkgs.system}`. Module switch done.

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

## Overseer Integration — Research Findings

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
  condition = { dir = "/path/to/project" },  -- optional: only show in certain dirs
  tags = { overseer.TAG.BUILD },              -- optional: tag for filtering
  desc = "Optional description",
}
```

**Key APIs:**
- `overseer.register_template(defn)` — register a template directly
- `overseer.run_task({name=..., tags=..., first=true}, callback)` — run a task programmatically
- `overseer.list_tasks()` — list all tasks
- `overseer.toggle()` — open/close the task list UI
- `overseer.add_template_hook(opts, hook_fn)` — modify template definitions at load time

**Template provider** (dynamic) — can use a `generator` function that returns tasks via callback, with `cache_key` for caching. But static `.lua` files are simpler.

**VS Code tasks.json** is also supported (feature-parity list in docs), but Lua templates are more flexible for forge's use case.

### forge + overseer integration plan

**Template location:** `~/.local/share/nvim/site/lua/overseer/template/forge/`  
This is under `.local`, not the nvim config dir — clean separation.

**Per-project templates:** One `.lua` file per indexed project. Filename matches project name (sanitized). Template reads the project's `.wl` at **runtime** (when overseer loads it), so it always reflects current build/run/test fields.

**Template regeneration:** `forge overseer --regen` iterates all projects in `index.json, reads each `.wl`, and writes/overwrites the corresponding `~/.local/share/nvim/site/lua/overseer/template/forge/<project>.lua`. Does not require nvim restart — overseer re-scans on `:OverseerRun`.

**Default task behavior:** The template has no `condition.dir` — it appears in `:OverseerRun` for all projects. Optional `forge overseer <name> --regen` for single-project regeneration.

**forge overseer command:**
- `forge overseer` — open overseer picker (runs `overseer.toggle()` equivalent via CLI)
- `forge overseer --regen` — regenerate all project templates
- `forge overseer <name>` — regenerate single project's template
- `forge overseer --rm <name>` — remove a project's template

**Template builder reads `.wl` at load time** (not at template-write time), so `.wl` changes are reflected on next `:OverseerRun` without regeneration.

### What needs to change

**`includes/overseer/setup.sh`** — currently writes `.vscode/tasks.json`. Must be rewritten to:
- Create `~/.local/share/nvim/site/lua/overseer/template/forge/` dir
- Write a per-project lua template that reads build/run/test from the project's `.wl`
- Remove `.vscode/tasks.json` approach entirely

**`src/commands/overseer.rs`** — currently a stub. Implement:
- `overseer --regen` → iterate index, write templates
- `overseer <name>` → write single project's template
- `overseer --rm <name>` → delete template file
- No-op if nvim not installed

**`includes/overseer/include.wl`** — update fields:
- `provides = ["overseer"]` (not "overseer-template")
- `requires = []` (overseer.nvim is a nvim plugin, not a system req)

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

### Overseer integration — IN PROGRESS
#### Research ✅
- [x] overseer.nvim template format (Lua files under `lua/overseer/template/`)
- [x] `overseer.register_template()`, `overseer.run_task()`, `overseer.toggle()`
- [x] Template builder reads `.wl` at load time (not at write time)
- [x] Template dirs: `~/.local/share/nvim/site/lua/overseer/template/forge/`

#### Implementation
- [ ] Rewrite `includes/overseer/setup.sh` — remove `.vscode/tasks.json`, write per-project Lua template to `~/.local/share/nvim/site/lua/overseer/template/forge/<name>.lua`
- [ ] Update `includes/overseer/include.wl` — `provides=["overseer"]`, `requires=[]`
- [ ] Implement `src/commands/overseer.rs`:
  - `forge overseer` → open overseer picker (no-op if no nvim)
  - `forge overseer --regen` → regenerate all project templates
  - `forge overseer <name>` → regenerate single project template
  - `forge overseer --rm <name>` → remove project template
- [ ] Add `forge overseer-def <name>` (already exists in `overseer_def.rs`, hook it up to CLI)

### ZSH completions — IN PROGRESS
- [ ] Write `completions/zsh/_forge`
- [ ] Subcommands: create, remove, list, sync, cd, session, pick, setup, include, lang, overseer, overseer-def, edit, open, help
- [ ] Flags per subcommand (--lang, --no-open, --dry-run, etc.)
- [ ] Language name and include name completions (from store dirs)
- [ ] Project name completions (from index.json)

### Waiting on Adam — Test end-to-end
- [ ] `forge create x --lang rust --no-open` — cargo init, git commit, direnv
- [ ] `forge create x --lang python --no-open` — poetry init via nix develop
- [ ] `forge create x --lang c --no-open` — generic lang
- [ ] `forge create x --lang rust --include git` — verify git remote set
- [ ] `forge list` — shows all created projects
- [ ] `forge cd x --print` — returns correct path
- [ ] `forge overseer --regen` — generates Lua templates
- [ ] `:OverseerRun` in nvim — shows forge tasks

### Missing features (low priority)
- [ ] Test `forge pick` ctrl-o opens nvim with Oil
- [ ] Test `forge session x --open` opens nvim with Oil

---

## Directory layout

```
forge/
├── Cargo.toml
├── flake.nix
├── src/
│   ├── main.rs, cli.rs
│   └── commands/
│       ├── create.rs, remove.rs, list.rs, sync.rs
│       ├── cd.rs, session.rs, pick.rs
│       ├── open.rs, setup.rs, lang.rs, include.rs
│       ├── overseer.rs        # stub — needs implementation
│       ├── overseer_def.rs    # exists, hook to CLI
│       └── edit.rs
├── languages/
│   ├── rust/setup.sh, flake.nix.template
│   ├── python/setup.sh, flake.nix.template
│   └── c, cpp, java, nix, r, txt/
├── includes/
│   ├── git/setup.sh
│   └── overseer/setup.sh      # needs rewrite for Lua templates
│       └── include.wl         # needs update
├── completions/
│   └── zsh/_forge             # missing — needs creation
├── module/default.nix
├── nix/package.nix
└── TODO.md
```