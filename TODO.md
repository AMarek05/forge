# forge

tmux sessionizer backed by `~/sync` with plugin-style includes and overseer.nvim integration.

**Fully buildable with Nix.** Ships with default language packs each containing a template `flake.nix` and scaffolding scripts.

---

## Concept

`forge` manages persistent tmux sessions for projects under `~/sync`. Each project has a `.wl` metadata file at its root. Projects declare a `language` (from the language registry) and optional `includes`. Language packs scaffold a per-project `flake.nix` + `.envrc`. Includes run setup scripts (git init, overseer, etc.) at creation time. The tool also serves as an fzf-powered session picker and an overseer task definition generator for nvim.

---

## Base Paths

| Variable | Default | Purpose |
|----------|---------|---------|
| `$FORGE_BASE` | `~/.forge` | Config, includes, languages |
| `$FORGE_SYNC_BASE` | `~/sync` | Where projects live |

---

## Directory layout

```
~/.forge/
├── config.sh              # global context (GitHub user, editor, tmux binary)
├── languages/             # language registry
│   ├── rust/
│   │   ├── lang.wl
│   │   ├── setup.sh
│   │   └── flake.nix.template
│   ├── r/
│   ├── python/
│   ├── java/
│   ├── cpp/
│   └── c/
├── includes/              # plugin-style include modules
│   ├── git/
│   │   ├── include.wl
│   │   └── setup.sh
│   └── overseer/
│       ├── include.wl
│       └── setup.sh
└── templates/            # future: full project templates

~/sync/                    # project root (FORGE_SYNC_BASE)
├── Code/
│   ├── Rust/
│   │   └── my-project/
│   │       ├── flake.nix
│   │       ├── .envrc
│   │       └── .wl
│   ├── R/
│   ├── Python/
│   ├── Java/
│   ├── C++/
│   └── C/
└── Notes/
    └── txt/
        └── my-notes/
            └── .wl
```

---

## Global Config: `~/.forge/config.sh`

Shell script sourced by all setup scripts. Defines global context.

```bash
FORGE_GITHUB_USER="AMarek05"
FORGE_DEFAULT_REMOTE_BASE="git@github.com:AMarek05"
FORGE_SYNC_BASE="$HOME/sync"
FORGE_BASE="$HOME/.forge"
FORGE_EDITOR="${EDITOR:-nvim}"
FORGE_TMUX_BINARY="${TMUX_BINARY:-tmux}"
FORGE_PATH_OVERRIDE=""    # if set, ignore lang.path — use this as shared root for all creates
```

---

## Language registry: `~/.forge/languages/<name>/`

Each language lives under `~/.forge/languages/<name>/` with a `lang.wl` metadata file, a `setup.sh` script, and optionally a `flake.nix.template`.

### `lang.wl` — language metadata

```bash
# ~/.forge/languages/rust/lang.wl
name="rust"
desc="Rust project with cargo and rustflake"
path="Code/rust"
direnv="use_flake"
requires=["cargo", "rustflake"]
setup_priority="10"

build="cargo build"
run="cargo run"
test="cargo test"
check="cargo check"
```

```bash
# ~/.forge/languages/txt/lang.wl
name="txt"
desc="Plain text notes — no flake, no toolchain"
path="Notes/txt"
direnv="none"
requires=[]
setup_priority="5"
```

| Field | Meaning |
|-------|---------|
| `name` | Language identifier (directory name) |
| `desc` | Human-readable description |
| `path` | Relative path under `FORGE_SYNC_BASE` where projects of this language live |
| `direnv` | Directive to emit in `.envrc`: `use_flake`, `use_poetry`, `layout go`, etc. Use `none` for no direnv |
| `requires` | Tools that must be in PATH for the setup to succeed |
| `setup_priority` | Order in which language setup runs vs includes (lower = earlier) |

### `flake.nix.template` — per-project flake template

Each language directory may contain a `flake.nix.template` file. When `forge create --lang rust my-project` runs, this is copied to the project as `flake.nix` with `{{PROJECT_NAME}}` substituted.

```nix
{
  description = "{{PROJECT_NAME}}";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { self, nixpkgs }: {
    devShells."${{SYSTEM}}".default = nixpkgs.legacyPackages."${{SYSTEM}}".mkShell {
      buildInputs = with nixpkgs.legacyPackages."${{SYSTEM}}"; [
        rustc
        cargo
        rustfmt
        clippy
      ];
    };
  };
}
```

### `setup.sh` — language scaffolding script

```bash
#!/bin/bash
# forge_description: Scaffold a Rust project with rustflake and direnv
# forge_requires: cargo, direnv, rustflake

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
  echo "[dry-run] cargo init $FORGE_PROJECT_PATH"
  echo "[dry-run] write .envrc"
  echo "[dry-run] write flake.nix"
  exit 0
fi

cd "$FORGE_PROJECT_PATH"

# Cargo init if not already a cargo project
if [ ! -f Cargo.toml ]; then
  cargo init .
fi

# Write .envrc
cat > .envrc << 'EOF'
use flake
EOF

# Write flake.nix (from template)
render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" > flake.nix

direnv allow
```

The `setup.sh` script:
1. Creates the project directory if needed
2. Writes `.envrc` with the `direnv` directive from `lang.wl`
3. Copies `flake.nix.template` → `flake.nix` with `{{PROJECT_NAME}}` substituted
4. Runs any language-specific init (cargo init, poetry init, etc.)
5. Runs `direnv allow`

### Standard language packs (shipped with forge)

| Language | Path | direnv | Toolchain |
|----------|------|--------|-----------|
| `rust` | `Code/Rust` | `use_flake` | cargo, rustc, rustflake |
| `r` | `Code/R` | `use_renv` | R, renv |
| `python` | `Code/Python` | `use_poetry` | poetry |
| `java` | `Code/Java` | `use_maven` | maven, java |
| `cpp` | `Code/C++` | `use_cmake` | cmake, g++ |
| `c` | `Code/C` | `use_make` | make, gcc |

---

## Special File: `.wl` (Workspace Launcher)

Key-value file at project root. No shebang, no sections.

```bash
# .wl — project metadata
name="my-project"                          # required; must be unique in index
lang="rust"                                # required; from language registry
desc="My awesome project"                   # optional
tags=["dev", "personal"]                   # optional
includes=["git", "overseer"]               # optional; include modules to apply

build="nix build"                          # optional; default overseer task
run="nix run"                              # optional
test="nix flake check"                     # optional

overseer_template="build"                  # optional; which task to run on session open

setup="bash ./scripts/dev-env.sh"          # optional; explicit setup script
```

**Line format rules:**
- `key="value"` → String
- `key=[array]` → Vec<String> (JSON array syntax)
- `#` prefix on a line → comment (ignored)
- blank lines → skipped
- unknown keys → ignored (forward-compatible)
- whitespace around `=` is stripped
- quotes (single/double) are stripped from values

**Validation:**
- `name` must be non-empty; must not contain `/` or null bytes
- `lang` must exist in language registry or error
- `includes` must be a valid JSON array of strings if present
- Duplicate key → last wins (warning to stderr)

---

## Include registry: `~/.forge/includes/<name>/`

Each include module provides fields (merged into `.wl`) and/or a setup script (run at creation).

### `include.wl` — include metadata

```bash
# ~/.forge/includes/git/include.wl
desc="Initialize git repo and set remote to GitHub"
provides=["git-init", "git-remote"]
requires=["git", "gh"]
version="1.0"

# No default fields — only runs setup.sh
```

```bash
# ~/.forge/includes/overseer/include.wl
desc="Register build/run/test tasks as overseer task definitions"
provides=["overseer-template"]
requires=["overseer.nvim"]
version="1.0"
```

### `setup.sh` — include setup script

```bash
#!/bin/bash
# forge_description: Initialize git repo and set GitHub remote
# forge_provides: git-init, git-remote
# forge_requires: git, gh

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
  echo "[dry-run] git init"
  echo "[dry-run] git remote add origin ..."
  exit 0
fi

cd "$FORGE_PROJECT_PATH"

if [ ! -d .git ]; then
  git init
fi

REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
```

**Contract:**
- Must be executable (`chmod +x`)
- Must have `# forge_description:` header
- Must have `# forge_provides:` header
- Must have `# forge_requires:` header
- Exit 0 on success; non-zero aborts the create/setup
- `--dry-run` mode: print actions without executing
- Runs with cwd = project root
- Has access to all `$FORGE_*` vars from `~/.forge/config.sh`

### Standard includes (shipped with forge)

| Include | Provides | Requires |
|---------|----------|----------|
| `git` | git-init, git-remote | git, gh |
| `overseer` | overseer-template | overseer.nvim |

---

## Creation flow: `forge create`

```
forge create my-project --lang rust
forge create my-project --lang rust --no-open
forge create my-project --lang rust --include git,overseer
forge create my-project --lang rust --setup
forge create my-project --lang rust --run "cargo test"
forge create my-project --lang rust --editor
```

**Steps:**
1. Resolve project path: `FORGE_SYNC_BASE/<lang.path>/<name>` → `~/sync/Code/rust/my-project`
2. Create directory if missing
3. Generate `.wl` from language defaults if absent (name, lang, desc, tags, includes, build/run/test/check fields)
4. Open `.wl` in `$EDITOR` (unless `--no-open`)
5. Run language `setup.sh` (if `direnv != "none"` and `setup.sh` exists)
6. Run each include in `includes` array order that has a `setup.sh`
7. Run `--run <cmd>` if passed (arbitrary command after all scaffolding done)
8. Run `--editor` (or `--run "$FORGE_EDITOR"`) if passed
9. Add project to index

| Flag | Effect |
|------|--------|
| `--no-open` | Skip `$EDITOR` (batch mode) |
| `--setup` | Force-run setup scripts after save |
| `--include <list>` | Pre-populate `includes` field |
| `--lang <name>` | **Required** — which language to use |
| `--path <path>` | Override project path; ignores `lang.path` for this create |
| `--run <cmd>` | Run arbitrary shell command after full creation |
| `--editor` | Open `$EDITOR` after full creation completes |
| `--dry-run` | Print all actions without executing |

---

## Path resolution

The project root for any new project is resolved in this order:

| Priority | Source | Example |
|----------|--------|---------|
| CLI flag | `--path <path>` | `forge create foo --path Work/special` → `~/sync/Work/special/foo` |
| Config var | `FORGE_PATH_OVERRIDE` in `config.sh` | `FORGE_PATH_OVERRIDE="Work"` → `~/sync/Work/<name>` for all creates |
| Language | `lang.path` from registry | `rust` → `~/sync/Code/Rust/<name>` |

`--path` always wins over `FORGE_PATH_OVERRIDE`, which always wins over `lang.path`.

## Per-project flakes

**Each project gets its own `flake.nix`. This is the recommended and default pattern.**

Per-project flakes mean:
- Store deduplication: same nixpkgs commit → same store path across all projects
- Full isolation: project-a can pin nixos-24.05, project-b can pin nixos-unstable
- Standard Nix idiom

A shared flake (one `flake.nix` at `Code/rust/flake.nix` used by all subdirs) is possible by customizing the language's `setup.sh`, but it is not the default and not recommended. The store overhead of per-project flakes is negligible.

---

## Index: `~/.forge-index.json`

Cached project list, re-built on `forge sync`.

```json
{
  "version": 1,
  "sync_base": "/home/adam/sync",
  "projects": [
    {
      "name": "nixos-conf",
      "lang": "nix",
      "path": "/home/adam/sync/Code/nix/nixos-conf",
      "desc": "NixOS flake config management",
      "tags": ["nixos", "config"],
      "includes": ["git"],
      "build": "nix build",
      "added_at": "2026-04-24T23:00:00Z",
      "last_opened": "2026-04-24T20:00:00Z",
      "open_count": 12
    }
  ]
}
```

---

## Commands

### `forge create [name] --lang <lang> [flags]`

Create a new project. See creation flow above.

### `forge remove <name>`

Remove from index. Does not delete files.

### `forge list [--tags <tags>]`

```
NAME          LANG  DESC                              TAGS
nixos-conf    nix   NixOS flake config management      nixos,config
dotfiles      txt   Personal dotfiles                  config,home
```

### `forge sync`

Re-scan `FORGE_SYNC_BASE` for all `.wl` files and rebuild index. Preserves `last_opened` and `open_count`.

### `forge cd <name>`

Print project path to stdout.

```bash
cd "$(forge cd nixos-conf)"
```

### `forge session [name] [--setup]`

Switch to or create a tmux session named `forge-<name>`.

```bash
forge session nixos-conf
forge session nixos-conf --setup
```

**Switch-or-create:**
1. `tmux has-session -t forge-<name>` → exists → `tmux switch-client -t forge-<name>`
2. Missing → `tmux new-session -d -s forge-<name> -c <path>` then `tmux switch-client -t forge-<name>`

### `forge pick [--tags <tags>]`

Interactive fzf picker.

```bash
forge pick
forge pick --tags nixos,config
```

**fzf layout:**
- Fuzzy matches on `name`, `desc`, `tags`
- Preview: `--preview 'cat {2}/.wl'` in right 60% window

**fzf keybindings:**

| Key | Action |
|-----|--------|
| `Enter` | Open session (switch-or-create) |
| `Ctrl+R` | Remove selected project |
| `Ctrl+E` | Edit `.wl` in `$EDITOR` |
| `Ctrl+O` | Open project in `$EDITOR` (no session) |
| `Ctrl+D` | Dry run |
| `Ctrl+S` | Toggle --setup flag |

### `forge setup <name>`

Run setup scripts for language + all includes.

```bash
forge setup nixos-conf
forge setup nixos-conf --dry-run
```

### `forge include --list`

List all available includes with description, provides, requires.

```
$ forge include --list

git        Initialize git repo and set GitHub remote
  provides: git-init, git-remote
  requires: git, gh

overseer   Register build/run/test as overseer tasks
  provides: overseer-template
  requires: overseer.nvim
```

### `forge include <name>`

Print full documentation for an include.

### `forge lang --list`

List all available languages with path and description.

```
$ forge lang --list

rust      Code/rust    Rust project with cargo and rustflake
nix       Code/nix     Nix flake project
txt       Notes/txt    Plain text notes
```

### `forge lang add [--interactive]`

Interactively scaffold a new language under `~/.forge/languages/`.

```bash
forge lang add                    # wizard
forge lang add crystal --path Code/crystal --direnv use_flake
```

Wizard steps:
1. Language name (directory name)
2. Path under `~/sync` (e.g., `Code/crystal`)
3. direnv directive (`use_flake`, `layout python`, etc., or `none`)
4. Required tools (comma-separated)
5. Default build/run/test commands
6. Generate `lang.wl` + `setup.sh` + `flake.nix.template`

### `forge overseer-def <name>`

Print JSON overseer task definition.

```json
{
  "name": "nixos-conf:build",
  "builder": "custom",
  "cmd": "nix build",
  "cwd": "/home/adam/sync/Code/nix/nixos-conf"
}
```

Exit 0 on success, 1 if not found.

### `forge edit <name>`

Open project's `.wl` in `$EDITOR`.

### `forge open <name>`

Open project directory in `$EDITOR`.

### `forge help [command]`

Print help.

---

## Project structure

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
├── languages/             # default language packs
│   ├── nix/
│   ├── rust/
│   ├── python/
│   ├── go/
│   ├── node/
│   └── txt/
├── includes/              # default includes
│   ├── git/
│   └── overseer/
├── completions/
│   └── zsh/
├── SPEC.md
└── README.md
```

---

## Nix build

### `flake.nix`

```nix
{
  description = "forge — tmux sessionizer with includes and overseer integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        packages = {
          default = pkgs.callPackage ./nix/package.nix { };
          forge = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
            clippy
          ];
        };
      }
    );
}
```

### `nix/package.nix`

```nix
{ stdenv, rustPlatform, pkg-config, openssl, dbus, fzf, direnv, nix }:

rustPlatform.buildRustPackage {
  pname = "forge";
  src = lib.cleanSource ./.;

  buildInputs = [ pkg-config openssl dbus fzf direnv nix ];

  postInstall = ''
    mkdir -p $out/share/forge/languages
    mkdir -p $out/share/forge/includes
    cp -r languages/* $out/share/forge/languages/
    cp -r includes/*  $out/share/forge/includes/
    cp completions/zsh/_forge $out/share/zsh/site-functions/
  '';
}
```

### Installation

```bash
# From the flake
nix profile install github:AMarek05/forge

# From local
cd forge && nix build && ./result/bin/forge --help
```

### Post-install init

```bash
forge init    # creates ~/.forge/config.sh and copies shipped languages/includes to ~/.forge/
```

Defaults ship at `$out/share/forge/`. `forge init` copies them to `~/.forge/` so the user can customize them.

---

## Include merge rules

| Situation | Behavior |
|-----------|----------|
| String field not set in project `.wl` | Inherit from include or language |
| String field already set in project `.wl` | Keep project value |
| Array field (`includes`, `tags`) | Concatenate, deduplicate |
| Unknown fields in include | Ignored |
| Duplicate key | Last wins, warning to stderr |

---

## Error checking

| Condition | Behavior |
|-----------|----------|
| `name` missing or invalid | Error: "name is required and must be a valid filename" |
| `lang` not in registry | Error: "language 'foo' not found in registry" |
| Include not found | Warning, continue |
| `tmux` not in PATH | Error: "tmux not found in PATH" |
| Setup script fails | Error with include name, abort create |
| Duplicate project in index | Warning, skip |
| `--dry-run` | Print actions without executing |

---

## Out of scope (v1)

- File system event auto-sync
- Remote SSH tmux sessions
- Multiple `.wl` per project
- Workspace groups
- Full project templates (includes only)
- `go`, `node`, `txt` languages (re-add via `forge lang add`)

---

## Shell completions

ZSH only. Shipped at `completions/zsh/_forge`.

Completions for: `create`, `remove`, `list`, `pick`, `session`, `setup`, `include`, `lang`, `edit`, `open` — covering project names, language names, include names, tag names, and all flags.
---

## Subagent phases

### Phase 1: Foundation (parallel, no inter-deps)

**Subagent A — Rust project scaffold**
- `Cargo.toml` with: clap, serde, serde_json, dirs, fzf, regex, anyhow, thiserror
- `src/main.rs` — basic clap CLI skeleton with all commands stubbed
- `src/cli.rs` — argument parsing (all commands + flags)
- `src/config.rs` — load `~/.forge/config.sh`
- `src/wl_parser.rs` — parse `.wl` and `lang.wl` files (key=value, arrays, comments)
- `src/index.rs` — read/write `~/.forge-index.json`
- `src/paths.rs` — path resolution (CLI flag > FORGE_PATH_OVERRIDE > lang.path)

**Subagent B — Nix build files**
- `flake.nix` — rust overlay, flake-utils, dev shell + packages
- `nix/package.nix` — derivation with buildInputs and postInstall (copies languages/, includes/, completions/)
- `devel/flake.nix` — self-referential dev shell: `inputs.self.outputs.packages`
- `devel/.envrc` — `use flake .#devShells.x86_64-linux.default`
- `rust-toolchain.toml` — pin to stable toolchain

**Subagent C — Language pack templates**
- 6x `languages/<lang>/lang.wl` — metadata (rust, r, python, java, cpp, c)
- 6x `languages/<lang>/setup.sh` — scaffolding scripts
- 6x `languages/<lang>/flake.nix.template` — per-project flake with `{{PROJECT_NAME}}` placeholder

**Phase 1 documentation pass**
- Verify all 3 subagent outputs against SPEC.md
- Update SPEC.md with any missed details discovered during implementation
- Write `docs/architecture.md` — module structure and key design decisions

---

### Phase 2: Core implementations (after Phase 1, can run parallel)

**Subagent D — create + tmux**
- `src/tmux.rs` — session switch-or-create logic
- `src/commands/create.rs` — full creation flow with path resolution
- `src/commands/session.rs`
- `src/commands/pick.rs` — fzf integration with keybindings

**Subagent E — includes + setup + lang**
- `src/include.rs` — registry reader + field merge logic
- `src/commands/setup.rs` — run include + language setup scripts
- `src/commands/include.rs` — `forge include --list` and `forge include <name>`
- `src/commands/lang.rs` — `forge lang --list` and `forge lang add <name> [flags]` (non-interactive)

**Subagent F — remaining commands**
- `src/commands/list.rs`, `sync.rs`, `remove.rs`, `cd.rs`
- `src/commands/overseer.rs` — JSON output for overseer.nvim
- `src/commands/edit.rs`, `open.rs`

**Phase 2 documentation pass**
- Review all command implementations against SPEC.md
- Update SPEC.md command docs to match actual flag names and behavior
- Write `docs/commands.md` — auto-generated command reference

---

### Phase 3: Completions + polish

**Subagent G — ZSH completions + init**
- `completions/zsh/_forge` — all commands, flags, project names, language names, include names
- `src/commands/init.rs` — `forge init` (copies shipped languages/includes to `~/.forge/`)
- Ensure all command help strings are complete and accurate

**Phase 3 documentation pass**
- Write `README.md` — installation, quick start, command reference
- Verify `completions/zsh/_forge` covers every command and flag
- Final SPEC.md sweep — all open questions resolved, all commands documented

---

### Spawning order

```
Phase 1: spawn A, B, C in parallel
         ↓ (wait for all 3 to complete)
Phase 2: spawn D, E, F in parallel
         ↓ (wait for all 3 to complete)
Phase 3: spawn G
         ↓ (wait for G to complete)
Done
```

I oversee each phase — review what subagent produced, verify against SPEC, resolve discrepancies before moving to next phase.
