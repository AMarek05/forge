# Restructuring Plan

**Goal:** Remove hardcoded `~/.forge` from binary, drop `config.sh` shell parsing, make config a structured file, remove hardcoded language list from module.
**Status:** Phase 1 ready. Phase 2 deferred.
**Prerequisite for all phases:** `nix build .#forge` must pass throughout.

---

## Phase 1 — Config file restructure
**Config dir set by HM via `FORGE_CONFIG_DIR`, binary reads from it. No flag, no fallback. Error if missing.

### 1.1 — Binary: `src/config.rs` — JSON config, error if not found

**Changes to `ForgeConfig`:**
Remove fields that are redundant or not needed:
- `base` — no longer needed (derive from `FORGE_CONFIG_DIR` at runtime)
- `default_remote_base` — can be computed at module level, not needed in config
- `path_override` — removed in new design

Keep:
- `github_user: String`
- `sync_base: PathBuf`
- `editor: String`
- `tmux_bin: String`
- `lang_dir: PathBuf`
- `include_dir: PathBuf`

**Changes to `load()`:**
```
FORGE_CONFIG_DIR env var → <config_dir>/config (JSON file)
Symlink followed automatically (Rust PathBuf resolves symlinks)
If env var absent → error: FORGE_CONFIG_DIR not set
If config file missing → error: config file not found at <path>
```
No fallback to hardcoded `~/.forge` — module is authoritative.

**JSON config format (written to `$FORGE_CONFIG_DIR/config` by HM):**
```json
{
  "sync_base": "/home/user/sync",
  "editor": "/run/current-system/sw/bin/nvim",
  "tmux_bin": "/run/current-system/sw/bin/tmux",
  "github_user": "AMarek05",
  "lang_dir": "/nix/store/<hash>-forge-languages",
  "include_dir": "/nix/store/<hash>-forge-includes"
}
```

### 1.2 — Module: write structured config, set one env var

**`module/default.nix`** changes:
1. Add `configDir` option with default `${config.home.homeDirectory}/.forge`:
   ```nix
   configDir = lib.mkOption {
     default = "${config.home.homeDirectory}/.forge";
     type = lib.types.path;
     description = "Directory where forge stores its config and runtime state";
   };
   ```
2. Write structured JSON config to `<configDir>/config` via `home.file`:
   ```nix
   home.file."${cfg.configDir}/config" = {
     text = builtins.toJSON {
       sync_base   = cfg.syncBase;
       editor      = cfg.editor;
       tmux_bin    = cfg.tmuxBinary;
       github_user = cfg.githubUser;
       lang_dir    = cfg.lang_dir;
       include_dir = cfg.include_dir;
     };
   };
   ```
   This writes the file directly to the host at build time — no symlink to store needed. Module owns this file.
3. Replace ALL `home.sessionVariables.FORGE_*` with a single:
   ```nix
   home.sessionVariables.FORGE_CONFIG_DIR = cfg.configDir;
   ```
4. Keep `home.file."${compDir}/_forge"` for zsh completion

### 1.3 — Binary: `src/config.rs` remove shell parsing, add JSON serde
Remove from `config.rs`:
- Regex for `export VAR="..."` pattern
- `parse()` function
- `unwrap_or()` fallback patterns for fields
- `base`, `default_remote_base`, `path_override` fields
Add:
- `serde` for JSON parsing (already in Cargo.toml)
- `load_from_path(config_dir: &Path) -> Result<Self>` that reads `<config_dir>/config` and parses with serde
- Helper methods: `index_path()`, `state_dir()` (both derived from config_dir)
- Helper method: `projects_dir()` → same as `sync_base`

### 1.4 — Binary: `src/paths.rs`
- `resolve_project_path` — no change needed, uses `config.sync_base`
- Add: `ForgeConfig::config_dir()` → returns `FORGE_CONFIG_DIR` resolved path
- Add: `ForgeConfig::index_path()` → returns `<config_dir>/index.json`
- Add: `ForgeConfig::state_dir()` → returns `<config_dir>/state/`
- Add: `ForgeConfig::projects_dir()` → returns `sync_base`

### 1.5 — Binary: update callers of `config.base` and field accesses
```
src/commands/create.rs   — config.base → config.config_dir() + index_path()
src/commands/list.rs     — config.base → config.config_dir() + index_path()
src/commands/remove.rs   — config.base → config.config_dir() + index_path()
src/commands/sync.rs     — config.base → config.config_dir() + index_path()
src/commands/health.rs   — config.base → config.config_dir() + index_path()
src/commands/check.rs    — uses config.lang_dir, config.include_dir (no change)
src/commands/overseer.rs — uses config.include_dir (no change)
src/commands/lang.rs     — uses config.lang_dir, config.sync_base (no change)
src/commands/cd.rs       — uses config.sync_base (no change)
src/commands/pick.rs     — uses config.sync_base (no change)
src/commands/session.rs  — uses config.sync_base (no change)
src/commands/edit.rs     — uses config.editor (no change)
```
`config.base` removed from `ForgeConfig` struct entirely.

### 1.6 — Module: rename `tmuxBinary` to `tmuxBin` in option name (keep JSON key as `tmux_bin` for consistency)

---

## Phase 2 — Languages and includes auto-generated
**Languages and includes discovered from structured JSON files in config dir, not from hardcoded inline Nix. Updated at build time by module, optionally via `forge sync --langs --includes` that re-runs generation.

### 2.1 — Module: generate languages.json and includes.json at build time
At module eval time, generate:
```
~/.forge/languages.json   — JSON array: [{name, desc, path, direnv, build, run, test, check, buildInputs}, ...]
~/.forge/includes.json   — JSON array: [{name, desc, provides}, ...]
```
Written via `home.file` like the config. Immutable at runtime.

### 2.2 — Binary: `forge sync --langs` and `--includes`
`forge sync --langs` re-generates language setup files from `languages.json` (read from config lang_dir at runtime, which is the store path generated at build time).
`forge sync --includes` similarly for includes.
The Nix store paths (`lang_dir`, `include_dir`) are already in the config — binary just re-runs generation from those source files at runtime, same logic as module build time.
