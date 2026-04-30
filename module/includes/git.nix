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
    REMOTE_URL="git@github.com:$FORGE_GITHUB_USER/$FORGE_PROJECT_NAME.git"
    git remote add origin "$REMOTE_URL" 2>/dev/null || git remote set-url origin "$REMOTE_URL"
  '';
}