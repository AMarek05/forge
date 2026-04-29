# Test Plan — forge

## Running the suite

```bash
# All tests
cd ~/projects/sync-launcher
bash tests/run.sh all

# Unit tests only (Rust)
cargo test --lib

# Integration tests (Rust binary wrapper → bash suite)
cargo test --test integration

# Shell/completion tests
bash tests/run.sh shell

# Individual suites
bash tests/run.sh unit    # → cargo test --lib (placeholder, units run via cargo)
bash tests/run.sh module # → nix eval (placeholder)
bash tests/run.sh integration
bash tests/run.sh shell
```

---

## Test suite structure

### 1. Unit tests (`cargo test --lib`)
**Coverage**: Pure functions in `src/` modules. Run as part of `cargo test --lib`.

| Module | What it tests | Test count |
|--------|---------------|------------|
| `wl_parser.rs` | `strip_quotes`, `parse_json_array`, `parse_wl`, `parse_lang_wl` | 18 |
| `applied_includes.rs` | `load`, `save`, `diff_applied` | 7 |
| `project_state.rs` | `from_wl`, `diff`, `save/load` roundtrip, `load` default | 5 |
| `index.rs` | `save_index_to/load_index_from` roundtrip, `new()` version | 3 |

**Total: ~33 unit tests**

**Key behaviors tested:**
- `strip_quotes`: double/single/no quotes, whitespace
- `parse_json_array`: empty, single, multi, whitespace, not-array, single-quote arrays
- `parse_wl`: all fields, empty arrays, comments ignored, malformed line → Err, unclosed bracket → Err, unquoted string → Err, duplicate key → last wins
- `parse_lang_wl`: full lang + optional fields absent
- `applied_includes::load`: file missing → empty vec, reads single/multi, ignores blank lines
- `applied_includes::save+load`: roundtrip
- `applied_includes::diff_applied`: returns new only, empty when all applied
- `ProjectState::from_wl`: all fields extracted correctly
- `ProjectState::diff`: detects changed fields, empty when identical
- `ProjectState::save/load`: roundtrip preserves all fields
- `ProjectState::load`: missing file → default state
- `ProjectIndex::new`: version 3, empty projects

### 2. Integration tests (`bash tests/run.sh integration`)
**Coverage**: Binary-level behavior end-to-end. Uses a real `forge` binary built from the current commit.

**Environment**: Isolated `$TMPDIR` with fake `$HOME`, no git credentials, no real projects.

**Test cases** (from `tests/integration/suite.sh`):
```
forge create --dry-run                      → exits 0, no files created
forge list (empty index)                   → says "no projects"
forge create test-rust --lang rust --no-open → .wl created at correct path
forge create test-python --lang python --no-open → .wl created
forge list (with projects)                → shows test-rust, test-python
forge cd test-rust --print                 → outputs correct absolute path
forge remove test-python                  → removed from list
forge create test-with-git --lang rust --no-open --include git → includes=[git] in .wl
forge overseer --regen                     → no-ops gracefully with no projects
forge sync (stale entry)                  → warns "removed: stale-project"
forge edit edit-test --no-open            → .forge/state written
forge sync (applied-includes)             → .forge/applied-includes created after sync
forge health                              → structured output with ✅/⚠️/❌
forge health --fix (stale entry)         → stale removed from index
```

### 3. Shell / completion tests (`bash tests/run.sh shell`)
**Coverage**: Binary flags, completion generation, help text.

```
forge --generate-completion zsh           → outputs create|remove|list
forge --generate-completion bash          → outputs create|remove|list
forge --generate-completion fish          → outputs create|remove|list
forge --generate-completion unknown-shell → says "Unsupported shell"
forge --help                              → lists all subcommands
forge --version                           → outputs "forge"
static _forge completion file             → loads in zsh without errors
forge overseer (no index)                → graceful exit, not a panic
```

---

## What is NOT covered by the test suite (known gaps)

- **No tests for `verify_and_diff`** — it calls external processes (include setup.sh scripts) and is tightly coupled to filesystem state. Covered indirectly by integration tests that run `forge edit`, `forge create --include git`.
- **No tests for `check.rs` via cargo test** — `check_wl` is tested via integration tests (suite.sh runs `forge check` on real projects). Unit-testable portion is tested indirectly through integration.
- **No tests for `config.rs`** — loads from environment/filesystem; indirectly tested via integration.
- **No tests for `include.rs`** — setup script generation; indirectly tested via integration with `--include git`.
- **No tests for `overseer.rs` (template generation)** — requires nvim + overseer plugin; tested via `forge overseer --regen` in integration but not asserting on output content.
- **Module tests** (`tests/module/`) are placeholder Nix eval checks, not implemented as runnable tests.

---

## CI / pre-commit checklist

```bash
# Fast pre-commit (unit only, ~5s)
cargo test --lib

# Full validation (~30s)
bash tests/run.sh all

# Before pushing
cargo test --lib && bash tests/run.sh all
```

---

## Adding new tests

**Unit tests** (`#[cfg(test)] mod tests` inside a `src/*.rs` module):
- Use `temp_wl(content)` helper → writes to temp file, returns `PathBuf`, caller cleans up
- Use `temp_project()` helper → creates `TempDir/.forge`, returns `PathBuf`
- All test modules live directly in the source file (not separate test files)
- Tests reference library items via `crate::<module>::<item>`

**Integration tests** (`tests/integration/suite.sh`):
- Add a test block with `info "test name"` + assertions
- Use `$FORGE` binary from `nix build .#forge --print-out-paths`
- Set up isolated `$HOME` via `mktemp -d`
- Always `trap cleanup EXIT`