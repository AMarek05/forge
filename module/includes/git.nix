{
  description = "Initialize git repo and set remote to GitHub";
  provides = [
    "git-init"
    "git-remote"
  ];
  setup = ''
    set -e
    cd "$FORGE_PROJECT_PATH"
    if [ ! -d .git ]; then git init; fi
    cat << EOF > .gitignore
    .forge/
    .direnv/
    EOF

    REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
    git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
    git commit -a -m "start: First commit for $FORGE_PROJECT_NAME"
  '';
}

