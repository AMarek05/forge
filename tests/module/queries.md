#!/usr/bin/env nix
# language: nix

## Home-Manager Module Tests

Tests for `module/default.nix` — verifies language flakes, setup.sh scripts,
and lang.wl files are correctly generated at module evaluation time.

Run with: nix eval --file tests/module/test-module.nix

---

### Module: language flake generation

#### generate-lang-flake — rust
```nix
{
  lang = rust-lang;

  flake = builtins.toFile "flake.nix" ''
    { description = "Rust project with cargo";
      inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
      outputs = { self, nixpkgs }:
        let
          system = "x86_64-linux";
          pkgs = import nixpkgs { inherit system; };
        in
        {
          packages.''${system}.default = pkgs.mkShell {
            name = "Rust project with cargo";
            buildInputs = with pkgs; [ rustc cargo rustfmt clippy ];
          };
          devShells.''${system}.default = self.packages.''${system}.default;
        };
    }
  '';

  evaluated = import (pkgs.writeText "flake.nix" flake) { };

  want = {
    evaluated.packages.x86_64-linux.default.buildInputs
      ? [ rustc cargo rustfmt clippy ];
  };
}
```
**expected**: rust flake builds mkShell with correct buildInputs

---

### Module: setup.sh generation

#### generate-lang-setup — rust creates cargo init
```nix
{
  setup = generate-lang-setup "rust" rust-lang;

  script = builtins.readFile setup;

  want = {
    script contains "cargo init";
    script contains "use flake";
    script contains "render_template";
  };
}
```
**expected**: rust setup.sh renders template, runs cargo init, direnv allow

---

#### generate-lang-setup — python runs poetry init
```nix
{
  setup = generate-lang-setup "python" python-lang;

  script = builtins.readFile setup;

  want = {
    script contains "poetry init";
    script contains "use flake";
  };
}
```
**expected**: python setup.sh runs poetry init under nix develop

---

#### generate-lang-setup — c has no special init
```nix
{
  setup = generate-lang-setup "c" c-lang;

  script = builtins.readFile setup;

  want = {
    script contains "render_template";
    script doesNotContain "cargo";
    script doesNotContain "poetry";
  };
}
```
**expected**: c setup.sh just renders flake template and direnv allow

---

#### generate-lang-setup — txt has direnv none
```nix
{
  lang = txt-lang;

  setup = generate-lang-setup "txt" txt-lang;

  script = builtins.readFile setup;

  want = {
    script contains "use flake";
  };
}
```
**expected**: txt still generates flake template, direnv allow runs

---

### Module: lang.wl generation

#### generate-lang-langwl — rust lang.wl content
```nix
{
  langwl = generate-lang-langwl "rust" rust-lang;

  content = builtins.readFile langwl;

  want = {
    content contains ''name="rust"'';
    content contains ''desc="Rust project with cargo"'';
    content contains ''path="Code/Rust"'';
    content contains ''direnv="use flake"'';
  };
}
```
**expected**: lang.wl has name, desc, path, direnv

---

#### generate-lang-langwl — all languages
```nix
{
  langs = [ "rust" "python" "c" "cpp" "java" "nix" "r" "txt" ];

  results = map (name: generate-lang-langwl name all-languages.${name}) langs;

  want = {
    all results are non-empty files;
    each contains name="<lang>";
  };
}
```
**expected**: all 8 languages produce valid lang.wl files

---

### Module: include generation

#### generate-include-setup — git sets remote
```nix
{
  setup = generate-include-setup "git" git-include;

  script = builtins.readFile setup;

  want = {
    script contains "git init";
    script contains "git remote";
    script contains "$FORGE_GITHUB_USER";
  };
}
```
**expected**: git setup.sh inits repo and adds GitHub remote

---

#### generate-include-setup — overseer writes Lua templates
```nix
{
  setup = generate-include-setup "overseer" overseer-include;

  script = builtins.readFile setup;

  want = {
    script contains "overseer/template/forge";
    script contains "parse_wl_field";
    script contains "write_template";
    script contains "overseer.TAG.BUILD";
  };
}
```
**expected**: overseer setup.sh parses .wl and writes Lua templates

---

#### generate-include-includewl — overseer has correct fields
```nix
{
  includewl = generate-include-includewl "overseer" overseer-include;

  content = builtins.readFile includewl;
  parsed = builtins.fromJSON content;

  want = {
    parsed.provides = ["overseer"];
    parsed.version = "1.0";
  };
}
```
**expected**: overseer include.wl has provides=["overseer"]

---

### Module: zsh completion

#### zsh-completion — contains all subcommands
```nix
{
  completion = zsh-completion;

  want = {
    completion contains "create";
    completion contains "remove";
    completion contains "list";
    completion contains "sync";
    completion contains "cd";
    completion contains "session";
    completion contains "pick";
    completion contains "setup";
    completion contains "include";
    completion contains "lang";
    completion contains "overseer";
    completion contains "overseer-def";
    completion contains "edit";
    completion contains "open";
  };
}
```
**expected**: completion file has all 14 subcommands

---

#### zsh-completion — create has correct flags
```nix
{
  completion = zsh-completion;

  want = {
    completion contains "--lang";
    completion contains "--no-open";
    completion contains "--include";
    completion contains "--path";
    completion contains "--run";
    completion contains "--dry-run";
  };
}
```
**expected**: create subcommand has all its flags in completion

---

### Module: homeManagerModules output

#### flake exports homeManagerModules.default
```nix
{
  outputs = import flake.nix { ... };

  want = {
    outputs.homeManagerModules ? "default";
    isFunction outputs.homeManagerModules.default;
  };
}
```
**expected**: flake.nix exports homeManagerModules.default as a module function

---

### Module eval: integration

#### module produces correct config
```nix
{
  pkgs = import nixpkgs { system = "x86_64-linux"; };

  module = import module/default.nix {
    config = { home = { homeDirectory = "/home/test"; }; };
    lib = nixpkgs.lib;
    pkgs = pkgs;
  };

  cfg = module.config.forge;

  want = {
    cfg.syncBase = "/home/test/sync";
    cfg.languages = ["rust" "python" "c" "cpp" "nix" "java" "r" "txt"];
    cfg.includes = ["git" "overseer"];
  };
}
```
**expected**: module evaluates with correct default config values