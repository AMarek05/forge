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