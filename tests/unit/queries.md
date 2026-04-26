#!/usr/bin/env nix
# language: rust

## Forge Unit Tests (wl_parser, index, config)

These tests verify the core parsing and data structures of forge.
Run with: cargo test --manifest-path projects/sync-launcher/Cargo.toml

---

### wl_parser: parse_wl

#### parse_wl — basic key=value
```nix
{
  WL = ''
    name="myproject"
    lang="rust"
  '';

  want = {
    name = Some "myproject";
    lang = Some "rust";
    desc = None;
    tags = [];
    includes = [];
    build = None;
    run = None;
    test = None;
    check = None;
  };
}
```
**expected**: parse_wl returns WlFile with name="myproject", lang="rust"

---

#### parse_wl — all fields
```nix
{
  WL = ''
    name="myproject"
    lang="rust"
    desc="A test project"
    tags=["rust","tool"]
    includes=["git","overseer"]
    build="cargo build"
    run="cargo run"
    test="cargo test"
    check="cargo clippy"
  '';

  want = {
    name = Some "myproject";
    lang = Some "rust";
    desc = Some "A test project";
    tags = ["rust", "tool"];
    includes = ["git", "overseer"];
    build = Some "cargo build";
    run = Some "cargo run";
    test = Some "cargo test";
    check = Some "cargo clippy";
  };
}
```
**expected**: all fields parsed correctly

---

#### parse_wl — comments and empty lines skipped
```nix
{
  WL = ''
    # comment
    name="test"

    # another comment

    lang="rust"
  '';

  want = {
    name = Some "test";
    lang = Some "rust";
  };
}
```
**expected**: comments and blank lines do not affect parsing

---

#### parse_wl — duplicate key takes last
```nix
{
  WL = ''
    name="first"
    name="second"
  '';

  want = {
    name = Some "second";
  };
}
```
**expected**: last occurrence of duplicate key wins

---

#### parse_wl — malformed line returns error
```nix
{
  WL = ''
    this is not a valid line
  '';

  want = error "failed to parse line";
}
```
**expected**: parse_wl returns Err

---

### wl_parser: parse_lang_wl

#### parse_lang_wl — full language definition
```nix
{
  FILE = ''
    name="rust"
    desc="Rust project with cargo"
    path="Code/Rust"
    direnv="use flake"
    requires=["git","cargo","direnv"]
    build=""
    run=""
    test=""
    check=""
  '';

  want = {
    name = "rust";
    desc = "Rust project with cargo";
    path = "Code/Rust";
    direnv = "use flake";
    requires = ["git", "cargo", "direnv"];
    build = None;
    run = None;
    test = None;
    check = None;
  };
}
```
**expected**: Language struct with all fields populated

---

### index: load_index / save_index

#### save and load round-trip
```nix
{
  setup = ''
    INDEX_FILE=$(mktemp)
    PROJECT_PATH=$(mktemp -d)
  '';

  INDEX = {
    version = 1;
    sync_base = "/home/user/sync";
    projects = [
      {
        name = "test-project";
        lang = "rust";
        path = "$PROJECT_PATH";
        desc = None;
        tags = [];
        includes = [];
        build = Some "nix build";
        added_at = "1234567890";
        last_opened = None;
        open_count = 0;
      }
    ];
  '';

  steps = [
    save_index INDEX to INDEX_FILE
    loaded = load_index from INDEX_FILE
  ];

  want = {
    loaded.version = 1;
    loaded.projects[0].name = "test-project";
    loaded.projects[0].lang = "rust";
  };
}
```
**expected**: save_index -> load_index yields identical index

---

#### load_index from non-existent file returns default index
```nix
{
  FILE = "/nonexistent/path/index.json";

  want = {
    version = 1;
    projects = [];
  };
}
```
**expected**: load_index returns default ProjectIndex when file missing

---

### config: parse

#### parse — basic export-style config
```nix
{
  CONFIG = ''
    export FORGE_GITHUB_USER="amarek05"
    export FORGE_SYNC_BASE="$HOME/sync"
    export FORGE_BASE="$HOME/.forge"
    export FORGE_EDITOR="nvim"
  '';

  want = {
    github_user = "amarek05";
    sync_base = <home>/sync;
    base = <home>/.forge;
    editor = "nvim";
  };
}
```
**expected**: config fields extracted, $HOME expanded

---

#### parse — bare (non-export) lines accepted
```nix
{
  CONFIG = ''
    FORGE_GITHUB_USER="amarek05"
  '';

  want = {
    github_user = "amarek05";
  };
}
```
**expected**: non-export lines parsed same as export lines

---

#### parse — comments and blank lines skipped
```nix
{
  CONFIG = ''
    # comment
    FORGE_GITHUB_USER="user"

    # another
    FORGE_EDITOR="vim"
  '';

  want = {
    github_user = "user";
    editor = "vim";
  };
}
```
**expected**: comments/blank lines ignored

---

### overseer: template generation

#### write_task_template — cmd with special chars escaped
```nix
{
  PROJECT = {
    name = "my-project";
    path = "/home/user/sync/my-project";
  };

  TASK = "build";
  CMD = "nix build --print-build-plan";
  TAG = "BUILD";

  TEMPLATE_DIR = mktemp -d;

  write_task_template dir=TEMPLATE_DIR project=PROJECT task=TASK cmd=CMD tag=TAG

  TEMPLATE_FILE = "$TEMPLATE_DIR/my-project_build.lua";

  content = read(TEMPLATE_FILE);

  want = {
    content.name = "my-project_build";
    content.builder.cmd = ["bash", "-c", "nix build --print-build-plan"];
    content.builder.cwd = "/home/user/sync/my-project";
    content.tags = [overseer.TAG.BUILD];
  };
}
```
**expected**: special chars in cmd escaped for Lua, template written to correct path

---

#### write_project_templates — creates build/run/check templates
```nix
{
  PROJECT = {
    name = "test";
    path = "/sync/test";
  };

  WL = ''
    name="test"
    lang="rust"
    build="cargo build"
    run="cargo run"
    test="cargo test"
  '';

  TEMPLATE_DIR = mktemp -d;
  write_project_templates PROJECT

  files = ls TEMPLATE_DIR

  want = {
    files = ["test_build.lua", "test_run.lua", "test_check.lua"];
  };
}
```
**expected**: three template files created with correct cmds from .wl

---

#### write_project_templates — falls back to defaults when .wl fields missing
```nix
{
  PROJECT = {
    name = "test";
    path = "/sync/test";
  };

  WL = ''
    name="test"
    lang="rust"
  '';
  # no build/run/test fields

  TEMPLATE_DIR = mktemp -d;
  write_project_templates PROJECT

  build_template = read "$TEMPLATE_DIR/test_build.lua"

  want = {
    build_template.builder.cmd = ["bash", "-c", "nix build"];
    run_template.builder.cmd = ["bash", "-c", "nix run"];
    check_template.builder.cmd = ["bash", "-c", "nix flake check"];
  };
}
```
**expected**: missing .wl fields use nix defaults

---

#### remove_project_templates — removes all three templates
```nix
{
  TEMPLATE_DIR = mktemp -d;
  touch "$TEMPLATE_DIR/test_build.lua"
  touch "$TEMPLATE_DIR/test_run.lua"
  touch "$TEMPLATE_DIR/test_check.lua"

  remove_project_templates "test"

  files = ls TEMPLATE_DIR

  want = { files = []; };
}
```
**expected**: all three template files deleted

---

### create: build_wl_content

#### build_wl_content — new project with includes
```nix
{
  NAME = "myproject";
  LANG = "rust";
  EXISTING_WL = None;
  INCLUDES = ["git", "overseer"];

  result = build_wl_content NAME LANG EXISTING_WL INCLUDES;

  want = ''
    name="myproject"
    lang="rust"
    includes=["git","overseer"]

  '';
}
```
**expected**: wl content has name, lang, includes

---

#### build_wl_content — preserves existing fields from .wl
```nix
{
  NAME = "myproject";
  LANG = "rust";
  EXISTING_WL = "/sync/myproject/.wl";  # exists with desc, tags
  INCLUDES = [];

  EXISTING = ''
    name="myproject"
    lang="rust"
    desc="My great project"
    tags=["notes"]
    build="echo built"
  '';

  result = build_wl_content NAME LANG EXISTING_WL INCLUDES;

  want = {
    result contains "desc=\"My great project\"";
    result contains "tags=[\"notes\"]";
    result contains "build=\"echo built\"";
  };
}
```
**expected**: existing desc, tags, build carried over to new .wl