# Restructuring Plan

**Phase 1:** ✅ Done — structured JSON config, single `FORGE_CONFIG_DIR` env var, config as store symlink.
**Phase 2:** In progress — langs/includes as store symlinks in `~/.forge/`, auto-generated `langs.json`/`includes.json`.

---

## Phase 2 — Language/include catalog with runtime sync

### Module changes

**`config.json`** now points to local dirs:
```json
{
  "sync_base": "/home/user/sync",
  "editor": "/.../nvim",
  "tmux_bin": "/.../tmux",
  "github_user": "AMarek05",
  "lang_dir": "/home/user/.forge/langs",
  "include_dir": "/home/user/.forge/includes"
}
```

**Directory layout:**
```
~/.forge/
├── config.json       ← written directly (text, not store symlink)
├── langs.json        ← generated at build time (catalog)
├── includes.json     ← generated at build time (catalog)
├── langs/
│   └── default → /nix/store/...-forge-languages   (symlink, HM-controlled)
├── includes/
│   └── default → /nix/store/...-forge-includes     (symlink, HM-controlled)
```

**`langs.json` format:**
```json
[
  {"name": "rust", "description": "...", "lang_wl": {"name":"rust",...}},
  ...
]
```

**`includes.json` format:**
```json
[
  {"name": "git", "description": "...", "provides": ["git"], "setup_sh": "#!/bin/bash\n..."},
  ...
]
```

**Module rebuild behavior:**
- Remove `langs/default` and `includes/default` symlinks, recreate from store
- Leave `custom/` subdirs untouched (user-created langs/includes go there)
- Regenerate `langs.json` and `includes.json` from current language/include definitions

### Binary changes

**`forge sync --langs`:**
- ✅ Scan `~/.forge/langs/default/<each>/lang.wl` and `~/.forge/langs/custom/<each>/lang.wl`
- ✅ Single parse via `parse_lang_wl()` (was: `parse_wl` + `parse_lang_wl`, double-read)
- ✅ Write `~/.forge/langs.json`

**`forge sync --includes`:**
- ✅ Scan `~/.forge/includes/default/<each>/` and `~/.forge/includes/custom/<each>/`
- ✅ Parse `include.wl` via `wl_parser::strip_quotes` + `parse_json_array` (was: `extract_field` naive string strip)
- ✅ Write `~/.forge/includes.json`

**`forge create` / `forge lang`:**
- ✅ `load_language()` checks `lang_dir/default/<lang>/lang.wl` then `lang_dir/custom/<lang>/lang.wl` (was: flat `lang_dir/<lang>/lang.wl`)

**`forge check`:**
- ✅ Validates lang references against `lang_dir/default/`
- ✅ Validates include references against `include_dir/default/`
- ⚠️ Note: only checks `default/` dirs, not `custom/` — may falsely report user-created langs/includes as unknown

**Unused helpers removed:** `config_dir()`, `index_path()`, `state_dir()`, `projects_dir()` — the Phase 1 design didn't use them and Phase 2 design doesn't need them either. `config.json` is written directly now.

### Build constraint
- `nix build .#forge` must pass throughout
- All 36 unit tests must continue to pass
