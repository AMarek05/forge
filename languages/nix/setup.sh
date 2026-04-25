#!/bin/bash
# forge_description: Scaffold a Nix flake project with direnv
# forge_requires: nix

set -e

render_template() {
    local src="$1"
    local dst="$2"
    sed "s/{{PROJECT_NAME}}/${FORGE_PROJECT_NAME}/g" "$src" > "$dst"
}

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] mkdir -p $FORGE_PROJECT_PATH"
    echo "[dry-run] write .envrc"
    echo "[dry-run] write flake.nix"
    echo "[dry-run] direnv allow"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH"

cd "$FORGE_PROJECT_PATH"

# Write .envrc
cat > .envrc << 'EOF'
use flake
EOF

# Write flake.nix (from template)
if [ -f "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" ]; then
    render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix
else
    # Minimal default flake if no template
    cat > flake.nix << 'FLKEOF'
{
  description = "{{PROJECT_NAME}}";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system}.default = pkgs.mkShell {
        name = "{{PROJECT_NAME}}";
        buildInputs = with pkgs; [
          nix
        ];
      };

      devShells.${system}.default = self.packages.${system}.default;
    };
}
FLKEOF
    sed -i "s/{{PROJECT_NAME}}/${FORGE_PROJECT_NAME}/g" flake.nix
fi

# Init git and commit flake.nix so nix develop can see it
if [ ! -d .git ]; then
    git init
    git add flake.nix
    git commit -m "init"
fi

direnv allow