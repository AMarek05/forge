# Restructuring Plan

**Goal:** Remove hardcoded `~/.forge` from binary, drop `config.sh` shell parsing, make config a structured file, remove hardcoded language list from module.

---

## Phase 1 — Config File Restructure

### Binary side

**`src/config.rs`**
- Replace `load()` with `load_from(config_path: &Path) -> Result<Self>`
- Parse a structured config file (JSON or TOML) instead of shell script
- Add `--config <path>` CLI flag, default: `$FORGE_CONFIG` env var, fallback to `~/.forge/config`
- Fields: `github_user`, `sync_base`, `editor`, `tmux_binary`, `lang_dir`, `include_dir`
- No shell evaluation, no regex for `export VAR="..."` — just clean structured parsing
- `base` (config directory itself) derived from config file location
- Remove `default_remote_base`, `path_override`, `path_override` from config (not needed)

**`src/cli.rs`**
- Add `--config <path>` flag to CLI args
- Remove `FORGE_PATH_OVERRIDE` env var usage entirely — not needed if config is structured

**`src/paths.rs`**
- No changes needed — already uses `config.sync_base` and `config.lang_dir`

### Module side

**`module/default.nix`**
- Remove `all-languages` inline definition (8 hardcoded language blocks)
- Add a `languages.data` file in the repo root that the module reads to get language definitions
  - Format: TOML or JSON — `[rust]`, `[python]`, etc. with their fields
  - Module reads this data file at eval time to generate language files
  - This becomes the single source of truth for both HM module generation AND runtime discovery
- Remove `all-includes` inline definition — same treatment
- HM module generates `~/.forge/config` (structured file) at build time
  - HM module only passes one env var to session: `FORGE_CONFIG` pointing to that file
  - Remove ALL other `FORGE_*` env vars from `home.sessionVariables`
- Config file written by module: `$HOME/.forge/config` (not to Nix store — use `home.file`)

### Config format
```json
{
  "github_user": "AMarek05",
  "sync_base": "/home/user/sync",
  "editor": "/run/current-system/sw/bin/nvim",
  "tmux_binary": "/run/current-system/sw/bin/tmux",
  "lang_dir": "/nix/store/...-forge-languages",
  "include_dir": "/nix/store/...-forge-includes"
}
```

### `nix/package.nix`
- No changes needed — already ships `languages/` and `includes/` to store, binary discovers from `lang_dir` at runtime

---

## Phase 2 — Binary Reads Config, No Hardcoded Defaults

**`src/config.rs`**
- `load()` reads from `$FORGE_CONFIG` env var if set, else `~/.forge/config`
- If config file missing: return error with helpful message — no fallback to hardcoded defaults
- Remove `unwrap_or_default()` fallbacks — config must be complete

---

## Phase 3 — Remove Hardcoded Language List from Module

**`languages/` → `languages.toml` (new data file)**
```toml
[rust]
description = "Rust project with cargo"
path = "Code/Rust"
direnv = "use flake"
buildInputs = ["rustc", "cargo", "rustfmt", "clippy"]

[python]
description = "Python project with poetry"
path = "Code/Python"
direnv = "use flake"
buildInputs = ["python311", "poetry"]
...
```

**`module/default.nix`**
- `builtins.fromTOML` (or fromJSON) to read `languages.toml` at module eval time
- Remove inline `rust-lang`, `python-lang`, etc. block definitions
- Remove inline `all-languages` attribute set
- Generate language files from the data file, no hardcoded attrs
- Remove `setup_priority` from generated `lang.wl` (already removed from `WlFile` earlier)
- Same treatment for `includes/` → `includes.toml`

---

## Phase 4 — Remove Hardcoded Language Discovery from Binary

**`src/commands/lang.rs`**
- Discover languages from `config.lang_dir` at runtime (already the case)
- No hardcoded list in binary
- `forge lang list` lists files in `lang_dir`, parses each `lang.wl`
- `forge lang rust` resolves from `lang_dir/rust/lang.wl`

---

## Files to change
```
src/config.rs          — parse JSON/TOML config, --config flag, remove config.sh reading
src/cli.rs             — add --config flag
module/default.nix     — read languages.toml/includes.toml, write config file, drop hardcoded lists
languages.toml         — NEW — structured language data (single source of truth)
includes.toml          — NEW — structured include data
TODO.md                — this file, update after each phase
```

## Files to delete
```
src/config.rs.old      — (if we keep a backup, no — just replace)
~/.forge/config.sh     — removed from runtime, replaced by config file written by HM
```

## Files to change (devel)
```
devel/flake.nix        — if it has hardcoded paths that need config_dir parameter
```
