#!/bin/bash
# forge_description: Scaffold an R project with renv
# forge_requires: R, renv

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

# Write flake.nix
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

direnv allow