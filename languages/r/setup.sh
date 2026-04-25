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

# Write .envrc
cat > .envrc << 'EOF'
use_renv
EOF

# Write flake.nix (from template)
render_template "$FORGE_LANG_TEMPLATE_DIR/flake.nix.template" flake.nix

direnv allow
