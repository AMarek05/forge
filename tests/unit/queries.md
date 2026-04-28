---

### applied_includes: load/save/diff helpers

#### load — returns empty vec when file missing
```nix
{
  setup = ''PROJECT_DIR=$(mktemp -d)'';
  call = ''applied_includes::load(&PathBuf::from(project_dir))'';
  want = ''Ok(vec![])'';
}
```
**expected**: load returns empty vec when .forge/applied-includes doesn't exist

#### load — reads single include
```nix
{
  setup = ''
    PROJECT_DIR=$(mktemp -d)
    mkdir "$PROJECT_DIR/.forge"
    echo "git" > "$PROJECT_DIR/.forge/applied-includes"
  '';
  call = ''applied_includes::load(&PathBuf::from(project_dir))'';
  want = ''Ok(vec!["git".to_string()])'';
}
```
**expected**: load reads and parses applied-includes correctly

#### diff_applied — returns new includes only
```nix
{
  setup = ''current = vec!["git", "overseer"]; applied = vec!["git"]'';
  call = ''applied_includes::diff_applied(&current, &applied)'';
  want = ''vec!["overseer"]'';
}
```
**expected**: diff returns only includes not yet applied

#### diff_applied — returns empty when all applied
```nix
{
  setup = ''current = vec!["git"]; applied = vec!["git"]'';
  call = ''applied_includes::diff_applied(&current, &applied)'';
  want = ''vec![]'';
}
```
**expected**: no new includes means empty diff

#### save — writes then load roundtrip
```nix
{
  setup = ''PROJECT_DIR=$(mktemp -d)'';
  call = ''
    let inc = vec!["git", "overseer"];
    applied_includes::save(&PathBuf::from(project_dir), &inc)?;
    applied_includes::load(&PathBuf::from(project_dir))
  '';
  want = ''Ok(vec!["git".to_string(), "overseer".to_string()])'';
}
```
**expected**: save then load returns the same includes

---

### project_state: load/save/from_wl/diff

#### from_wl — extracts all fields
```nix
{
  setup = ''
    let wl = WlFile {
      name: Some("myproject".into()),
      lang: Some("rust".into()),
      desc: Some("A test project".into()),
      tags: vec!["cli", "wasm"],
      includes: vec!["git"],
      build: Some("cargo build".into()),
      run: Some("cargo run".into()),
      test: Some("cargo test".into()),
      check: Some("cargo clippy".into()),
      overseer_template: None,
      setup: None,
    };
  '';
  call = ''ProjectState::from_wl(&wl, 1234567890)'';
  want = ''ProjectState { name: "myproject".into(), lang: "rust".into(), desc: "A test project".into(), tags: vec!["cli", "wasm"], includes: vec!["git"], build: "cargo build".into(), run: "cargo run".into(), test: "cargo test".into(), check: "cargo clippy".into(), last_wl_mtime: 1234567890 }'';
}
```
**expected**: from_wl correctly populates all fields from WlFile

#### diff — detects changed fields
```nix
{
  setup = ''
    let old = ProjectState {
      name: "myproject".into(),
      lang: "rust".into(),
      desc: "old desc".into(),
      tags: vec![],
      includes: vec![],
      build: "cargo build".into(),
      run: "".into(),
      test: "".into(),
      check: "".into(),
      last_wl_mtime: 0,
    };
    let new = ProjectState {
      name: "renamed".into(),
      lang: "rust".into(),
      desc: "new desc".into(),
      tags: vec!["cli"],
      includes: vec!["git"],
      build: "cargo build".into(),
      run: "".into(),
      test: "".into(),
      check: "".into(),
      last_wl_mtime: 999,
    };
  '';
  call = ''old.diff(&new)'';
  want = ''vec!["name", "desc", "tags", "includes", "last_wl_mtime"]'';
}
```
**expected**: diff correctly identifies changed field names

#### diff — empty when identical
```nix
{
  setup = ''same = ProjectState { name: "test".into(), lang: "rust".into(), desc: "".into(), tags: vec![], includes: vec![], build: "".into(), run: "".into(), test: "".into(), check: "".into(), last_wl_mtime: 0 }'';
  call = ''same.diff(&same)'';
  want = ''vec![]'';
}
```
**expected**: diff of identical states is empty

#### save and load — roundtrip
```nix
{
  setup = ''PROJECT_DIR=$(mktemp -d)'';
  call = ''
    let state = ProjectState {
      name: "myproject".into(),
      lang: "rust".into(),
      desc: "A cool project".into(),
      tags: vec!["cli", "wasm"],
      includes: vec!["git", "overseer"],
      build: "cargo build".into(),
      run: "cargo run".into(),
      test: "cargo test".into(),
      check: "cargo clippy".into(),
      last_wl_mtime: 1234567890,
    };
    state.save(&PathBuf::from(project_dir))?;
    ProjectState::load(&PathBuf::from(project_dir))
  '';
  want = ''Ok(state)'';
}
```
**expected**: save then load returns identical state

---

### check: validate .wl syntax and field integrity

#### check_wl — valid .wl returns no errors
```nix
{
  FILE = ''
    name="test"
    lang="rust"
    desc="A test project"
    tags=["cli"]
    includes=[]
    build="cargo build"
  '';

  want = {
    errors = [];
    warnings = [];
  };
}
```
**expected**: check_wl returns empty errors and warnings for valid .wl

---

#### check_wl — syntax error returns line number
```nix
{
  FILE = ''
    name="test"
    lang="rust"
    this line is malformed
  '';

  want = {
    errors.length = 1;
    errors[0].line = 3;
    errors[0].msg contains "malformed line";
  };
}
```
**expected**: error at line 3, message mentions "malformed line"

---

#### check_wl — unclosed bracket is syntax error
```nix
{
  FILE = ''
    name="test"
    tags=["cli
  '';

  want = {
    errors.length = 1;
    errors[0].line = 2;
    errors[0].msg contains "unclosed";
  };
}
```
**expected**: error at line 2, message mentions unclosed bracket/string

---

#### check_wl — unknown lang produces error
```nix
{
  FILE = ''
    name="test"
    lang="rustt"
  '';

  want = {
    errors.length = 1;
    errors[0].msg contains "no such language";
  };
}
```
**expected**: error mentions unknown lang `rustt`

---

#### check_wl — unknown include produces error
```nix
{
  FILE = ''
    name="test"
    includes=["overseerr"]
  '';

  want = {
    errors.length = 1;
    errors[0].msg contains "no such include";
  };
}
```
**expected**: error mentions unknown include `overseerr`

---

#### check_wl — empty build/run/test/check produces warning
```nix
{
  FILE = ''
    name="test"
    lang="rust"
    build=""
    run=""
    test=""
    check=""
  '';

  want = {
    errors = [];
    warnings.length = 4;
  };
}
```
**expected**: warnings for all four empty command fields

---

#### check_wl — duplicate key produces error
```nix
{
  FILE = ''
    name="test"
    name="test2"
  '';

  want = {
    errors.length = 1;
    errors[0].msg contains "duplicate";
  };
}
```
**expected**: error mentions duplicate key `name`

---

#### check_wl — unquoted string value is syntax error
```nix
{
  FILE = ''
    name=test
  '';

  want = {
    errors.length = 1;
    errors[0].line = 1;
    errors[0].msg contains "quoted";
  };
}
```
**expected**: error at line 1, string value must be quoted

---

#### check_wl — empty array is valid
```nix
{
  FILE = ''
    name="test"
    includes=[]
    tags=[]
  '';

  want = {
    errors = [];
    warnings = [];
  };
}
```
**expected**: empty arrays are valid, no errors or warnings

---

#### check_wl — multiple tags in array
```nix
{
  FILE = ''
    name="test"
    tags=["rust","wasm","cli"]
  '';

  want = {
    errors = [];
    warnings = [];
  };
}
```
**expected**: valid multi-entry tag array, no issues

---

#### run_all — valid projects all pass
```nix
{
  setup = ''
    INDEX_FILE=$(mktemp)
    PROJECT_PATH=$(mktemp -d)
    echo 'name="p"' > "$PROJECT_PATH/.wl"
  '';

  want = {
    exit_code = 0;
    stdout contains "✅";
  };
}
```
**expected**: all projects pass, no errors printed

---

#### run_all — project with syntax error exits non-zero
```nix
{
  setup = ''
    PROJECT_PATH=$(mktemp -d)
    echo 'name=malformed' > "$PROJECT_PATH/.wl"
  '';

  want = {
    exit_code != 0;
    stderr contains "❌";
  };
}
```
**expected**: non-zero exit, error indicator printed