#!/bin/bash
# forge_description: Register build/run/test as overseer.nvim task definitions
# forge_provides: overseer-template
# forge_requires: overseer.nvim

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] write .vscode/tasks.json (overseer integration)"
    exit 0
fi

mkdir -p "$FORGE_PROJECT_PATH/.vscode"

# Write VS Code tasks.json as overseer-compatible task definitions
cat > "$FORGE_PROJECT_PATH/.vscode/tasks.json" << 'TASKSEOF'
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "{{PROJECT_NAME}}:build",
      "type": "shell",
      "command": "nix build",
      "problemMatcher": [],
      "group": "build"
    },
    {
      "label": "{{PROJECT_NAME}}:run",
      "type": "shell",
      "command": "nix run",
      "problemMatcher": [],
      "group": "run"
    },
    {
      "label": "{{PROJECT_NAME}}:check",
      "type": "shell",
      "command": "nix flake check",
      "problemMatcher": [],
      "group": "test"
    }
  ]
}
TASKSEOF

sed -i "s/{{PROJECT_NAME}}/${FORGE_PROJECT_NAME}/g" "$FORGE_PROJECT_PATH/.vscode/tasks.json"