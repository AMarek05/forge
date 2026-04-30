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
- `tmux_binary: String`  
- `lang_dir: PathBuf`
- `include_dir: PathBuf`
- `index_path: PathBuf` (derived from config dir / index.json)
- `state_dir: PathBuf` (derived from config dir / state/)
- `projects_dir: PathBuf` (same as sync_base)

**Changes to `load()`:**
```
FORGE_CONFIG_DIR env var → <config_dir>/config (JSON file)
Symlink followed automatically by Rust's PathBuf → canonical paths resolved
If env var absent → error with message pointing to the missing `FORGE_CONFIG_DIR`
If config file missing → error with message
```
No fallback to hardcoded `~/.forge` — module is authoritative

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
Naming: `tmux_bin` over `tmux_binary` to avoid "binary" being a config schema concern
Naming: no underscore in `tmux_bin` — short, consistent with `github_user`
Naming: `sync_base` matches HM module's option name exactly

### 1.2 — Module: write structured config, set one env var

**`module/default.nix`** changes:
1. Write `home.file.".forge/config"`.source = some store path — config written by module
2. Write it as a symlink to a generated store path:
   ```nix
   home.file.".forge/config" = {
     source = pkgs.runCommandNoCCM "forge-config" {
       content = builtins.toJSON {
         sync_base   = cfg.syncBase;
         editor      = cfg.editor;
         tmux_bin    = cfg.tmuxBinary;
         github_user = cfg.githubUser;
         lang_dir    = cfg.lang_dir;
         include_dir = cfg.include_dir;
       };
       passAsFile   = [ "content" ];
     } /share/forge/config;
   }
   ```
   Actually simpler: `home.file.".forge/config" = { text = builtins.toJSON { ... }; };` — HM writes file directly to host, no symlink needed. File owner is module. Binary reads it.

3. Remove ALL `home.sessionVariables` forge env vars — replace with:
   ```nix
   home.sessionVariables.FORGE_CONFIG_DIR = "${config.home.homeDirectory}/.forge";
   ```
4. Keep `home.file."${compDir}/_forge"` for zsh completion

### 1.3 — Binary: remove `src/config.rs` env var parsing
Remove from `config.rs`:
- Regex for `export VAR="..."` pattern
- `parse()` function
- `unwrap_or()` fallback patterns for fields
- `path_override`
- `base` field
- `default_remote_base`
Add:
- `serde` for JSON parsing
- `load_from_path(config_dir: &Path) -> Result<Self> that reads `<config_dir>/config` and parses with serde

### 1.4 — Binary: `src/cli.rs` changes  
No `--config-dir` flag. No `--config` flag. `load()` reads from `FORGE_CONFIG_DIR` only. CLI args unchanged.

### 1.5 — Binary: `src/paths.rs`
- `resolve_project_path` — no change needed, uses `config.sync_base`
- New helper: `ForgeConfig::index_path()` returns `<config_dir>/index.json`
- New helper: `ForgeConfig::state_dir()` returns `<config_dir>/state/`
- New helper: `ForgeConfig::config_dir()` returns `FORGE_CONFIG_DIR` resolved path

### 1.6 — Binary: update callers of `ForgeConfig::load()` and field accesses
```
src/lib.rs               — no change
src/commands/check.rs    — uses config.lang_dir, config.include_dir — no field names change
src/commands/create.rs   — uses config.base, config.sync_base, config.editor, config.tmux_binary
                            → derive base from config_dir, remove config.base uses
src/commands/overseer.rs — uses config.include_dir
src/commands/lang.rs    — uses config.lang_dir, config.editor, config.sync_base
src/commands/list.rs    — uses config.base
src/commands/edit.rs    — uses config.editor
src/commands/remove.rs  — uses config.base  
src/commands/sync.rs   — uses config.sync_base, config.base, config.lang_dir, config.include_dir
src/commands/session.rs — uses config.sync_base
src/commands/cd.rs      — uses config.sync_base
src/commands/pick.rs    — uses config.editor, config.sync_base
src/commands/health.rs  — uses config.base
```
No command changes beyond swapping `config.base` usage for `config_dir()/index_path()` etc.

---

## Phase 2 — Languages and includes auto-generated
**Languages and includes discovered from structured JSON files in config dir, not from hardcoded inline Nix. Updated at build time by module, optionally via `forge sync --langs --includes` that re-runs generation.
  
### 2.1 — Module: languages.toml / includes.toml replaced by structured JSON files

Module generates at build time:
```
~/.forge/languages.json   — [{name, desc, path, direnv, build, run, test, check, buildInputs}, ...]
~/.forge/includes.json   — [{name, desc, provides}, ...]
```

These are store paths symlinked by HM at build time. Immutable at runtime — to update, rebuild with module changes or run `forge sync --langs --includes` which triggers module's generation logic at runtime. How? Either:
- a) `forge sync --langs` triggers a nix derivation rebuild (not great for UX)
- b) `forge sync --langs` re-generates from the data file directly in the binary (reads from repo's source `languages/` directory at runtime)
- c) `forge sync --langs` re-generates from the Nix module's output (store path accessible at runtime via `lang_dir` path)
  
User clarification needed: does `forge sync --langs` need to trigger a nix rebuild? If not, option (b) — binary re-generates from repo source at runtime — seems cleanest.

### 2.2 — Binary: `src/wl_parser.rs` — remove hardcoded Language struct defaults
Current `Language` struct has defaults baked in for missing fields. Phase 1 config already removes `setup_priority` from `WlFile`, but `Language` still had `setup_priority`, `overseer_template`, `setup` — already removed per prior work. Check that binary doesn't assume any hardcoded language list exists anywhere. `src/commands/lang.rs` should read from `config.lang_dir` and parse `lang.wl` from each directory.

---

## Files to change (Phase 1 only)
```
src/config.rs          — remove regex/shell parsing, add serde JSON parsing, use FORGE_CONFIG_DIR env var
src/cli.rs             — no changes needed (no flag added in final design)
src/paths.rs           — add helper methods: config_dir(), index_path(), state_dir()
src/commands/check.rs  — no field name changes
src/commands/create.rs  — remove config.base usage, derive from config_dir()  
src/commands/overseer.rs — no field name changes
src/commands/lang.rs   — no field name changes
src/commands/list.rs   — swap config.base → config_dir() + index.json  
src/commands/edit.rs   — no field name changes
src/commands/remove.rs — swap config.base → config_dir() + index.json
src/commands/sync.rs   — swap config.base → config_dir() + index.json
src/commands/session.rs — no field name changes  
src/commands/cd.rs     — no field name changes
src/commands/pick.rs  — no field name changes
src/commands/health.rs — swap config.base → config_dir() + index.json
module/default.nix     — write JSON config to ~/.forge/config, set FORGE_CONFIG_DIR, remove FORGE_* sessionVariables
Cargo.toml             — add serde, serde_json for JSON config parsing
```
