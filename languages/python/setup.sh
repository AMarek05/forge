#!/bin/bash
# forge_description: Scaffold a Python project with poetry
# forge_requires: poetry

set -e

render_template() {
    local src="$1"
    local dst="$2"
    sed "s/{{PROJECT_NAME}}/${FORGE_PROJECT_NAME}/g" "$src" > "$dst"
}

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] mkdir -p $FORGE_PROJECT_PATH"
    echo "[dry-run] nix develop . -c poetry init --name $FORGE_PROJECT_NAME"
    echo "[dry-run] write .envrc"
    echo "[dry-run] write flake.nix"
    echo "[dry-run] direnv allow"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH"

cd "$FORGE_PROJECT_PATH"

# Write .envrc
cat > .envrc << 'EOF'
use poetry
EOF

# Write flake.nix (from template)
render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

# Init via nix develop so poetry is available even without global install
if [ ! -f pyproject.toml ]; then
    nix develop . -c poetry init --name "$FORGE_PROJECT_NAME" --quiet
fi

direnv allow
