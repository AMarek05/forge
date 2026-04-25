# forge — Architecture

## Module structure

```
forge/src/
├── main.rs           # entry point — parses CLI, dispatches commands
├── cli.rs            # clap enum: all commands and flags (derive macro)
├── config.rs         # ForgeConfig — parsed from ~/.forge/config.sh
├── wl_parser.rs      # WlFile — parses .wl and lang.wl key=value files
├── index.rs          # ProjectIndex — read/write ~/.forge-index.json
├── paths.rs          # resolve_project_path() — CLI flag > FORGE_PATH_OVERRIDE > lang.path
├── commands.rs       # re-exports all command modules
└── commands/
    ├── create.rs     # forge create — path resolution, .wl generation, setup scripts
    ├── remove.rs     # forge remove — remove from index
    ├── list.rs       # forge list — table output, tag filtering
    ├── sync.rs       # forge sync — re-scan ~/sync for .wl files
    ├── cd.rs         # forge cd — print path to stdout
    ├── session.rs    # forge session — switch-or-create tmux session
    ├── pick.rs       # forge pick — fzf interactive picker
    ├── setup.rs      # forge setup — run include/language setup scripts
    ├── include.rs    # forge include --list / <name> — include registry docs
    ├── lang.rs       # forge lang --list / add — language registry
    ├── overseer_def.rs # forge overseer-def — JSON task definition
    ├── edit.rs       # forge edit — open .wl in $EDITOR
    ├── open.rs       # forge open — open project dir in $EDITOR
    └── help.rs       # forge help — command-level help
```

## Key design decisions

### Path resolution

```
1. --path <path>           (CLI flag — always wins)
2. FORGE_PATH_OVERRIDE     (config.sh — per-create override)
3. lang.path               (language registry default)
```

`resolve_project_path(name, lang, config, explicit_path)` implements this in `paths.rs`.

### Config parsing (`config.rs`)

`~/.forge/config.sh` is a shell script with `export KEY="value"` lines. Parsed line-by-line with regex. `$HOME` is expanded in path values. Unknown keys are ignored.

### `.wl` format (`wl_parser.rs`)

Two line formats:
- `key="value"` → String (quotes stripped)
- `key=[array]` → Vec<String> (JSON array, whitespace stripped)

Lines starting with `#` are comments. Blank lines are skipped. Unknown keys are ignored. Duplicate keys: last wins with stderr warning.

### Index (`index.rs`)

`~/.forge-index.json` is versioned (`"version": 1`). Paths stored as `PathBuf`. `load_index()` returns `ProjectIndex`. `save_index()` writes atomically (write to temp, rename).

### Tmux session naming

All sessions named `forge-<name>` (prefix prevents collisions). Switch-or-create:
1. `tmux has-session -t $SESSION` → exists → `tmux switch-client -t $SESSION`
2. Missing → `tmux new-session -d -s $SESSION -c $PATH` then `tmux switch-client -t $SESSION`

### Language setup execution order

On `forge create`:
1. Resolve path (from lang.path or override)
2. Create directory
3. Write .wl (language defaults merged)
4. Open .wl in $EDITOR (unless `--no-open`)
5. Run language `setup.sh` (if `direnv != "none"`)
6. Run each include `setup.sh` in `includes` array order
7. Run `--run <cmd>` if passed
8. Run `--editor` if passed
9. Add to index

### Include merge rules

| Field type | Behavior |
|------------|----------|
| String not set in project | Inherit from include/language |
| String already set in project | Keep project value |
| Array (`includes`, `tags`) | Concatenate, deduplicate |
| Unknown key in include | Ignored |
| Duplicate key | Last wins, stderr warning |

### Language packs (`languages/<lang>/`)

Each language directory contains:
- `lang.wl` — metadata (name, desc, path, direnv, requires, build/run/test/check)
- `setup.sh` — scaffolding script (writes .envrc, renders flake.nix.template, runs direnv allow)
- `flake.nix.template` — per-project flake template with `{{PROJECT_NAME}}` placeholder

Default languages: rust, r, python, java, cpp, c.

### Nix build

```
flake.nix
└── pkgs.callPackage ./nix/package.nix { }
    └── rustPlatform.buildRustPackage
        ├── buildInputs: [pkg-config, openssl, dbus, fzf, direnv, nix]
        └── postInstall: copy languages/, includes/, completions/zsh/ to share/
```

Dev shell (devel/.envrc → `use flake .#devShells.x86_64-linux.default`):
- Self-referential flake at `devel/flake.nix` uses `inputs.self.url = "path:."`
- Pulls `devShells.default` from main flake via `inputsFrom`

## Out of scope

- File system event auto-sync
- Remote SSH tmux sessions
- Multiple `.wl` per project
- Workspace groups
- Full project templates
- `go`, `node`, `txt` languages (re-add via `forge lang add`)
- Interactive `forge lang add --interactive` wizard