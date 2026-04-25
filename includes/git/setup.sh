#!/bin/bash
# forge_description: Initialize git repo and set GitHub remote
# forge_provides: git-init, git-remote
# forge_requires: git, gh

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] git init"
    echo "[dry-run] git remote add origin git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
    exit 0
fi

cd "$FORGE_PROJECT_PATH"

if [ ! -d .git ]; then
    git init
fi

REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"