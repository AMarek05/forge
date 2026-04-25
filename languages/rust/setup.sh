#!/bin/bash
# forge_description: Scaffold a Rust project with cargo and rustflake
# forge_requires: cargo, rustflake

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
    echo "[dry-run] nix shell .#default -c cargo init ."
    echo "[dry-run] direnv allow"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH"

cd "$FORGE_PROJECT_PATH"

# Write flake.nix first (needed for nix shell)
render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

# Write .envrc
cat > .envrc << 'EOF'
use flake
EOF

# Use nix shell to get cargo without evaluating the project flake first
if [ ! -f Cargo.toml ]; then
    nix shell .#default -c cargo init .
fi

direnv allow
