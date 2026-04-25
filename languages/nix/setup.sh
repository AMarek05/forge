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
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { self, nixpkgs }: {
    devShells."${builtins.replaceStrings ["$"] [""] "$\{SYSTEM}"}".default = nixpkgs.legacyPackages."${builtins.replaceStrings ["$"] [""] "$\{SYSTEM}"}".mkShell {
      buildInputs = with nixpkgs.legacyPackages."${builtins.replaceStrings ["$"] [""] "$\{SYSTEM}"}"; [
        nix
      ];
    };
  };
}
FLKEOF
    sed -i "s/{{PROJECT_NAME}}/${FORGE_PROJECT_NAME}/g" flake.nix
fi

direnv allow