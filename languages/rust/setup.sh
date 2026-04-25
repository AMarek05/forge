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
    echo "[dry-run] git init"
    echo "[dry-run] write .gitignore"
    echo "[dry-run] git add flake.nix"
    echo "[dry-run] write .envrc"
    echo "[dry-run] nix develop . -c cargo init ."
    echo "[dry-run] direnv allow"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH"

cd "$FORGE_PROJECT_PATH"

# Write flake.nix first
render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

# Init git and commit flake.nix so nix develop can see it
if [ ! -d .git ]; then
    git init
    git add flake.nix
    git commit -m "init"
fi

# Write .envrc so future sessions get the flake environment via direnv
cat > .envrc << 'EOF'
use flake
EOF

# Init via nix develop so cargo is available even without global install
if [ ! -f Cargo.toml ]; then
    nix develop . -c cargo init .
fi

# Activate direnv for subsequent sessions
direnv allow