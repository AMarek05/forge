#!/bin/bash
# forge_description: Scaffold a plain text notes project — no toolchain
# forge_provides: txt-notes
# forge_requires:

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] mkdir -p $FORGE_PROJECT_PATH"
    echo "[dry-run] write .wl marker file"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH"

cd "$FORGE_PROJECT_PATH"

# Plain text — no .envrc, no flake.nix, just mark the project type
touch ".txt-marker"

direnv allow