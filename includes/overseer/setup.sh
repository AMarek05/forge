#!/bin/bash
# forge_description: Add overseer.nvim task templates for the project
# forge_provides: overseer
# forge_provides: 

set -e

if [ "$FORGE_DRY_RUN" = "1" ]; then
    echo "[dry-run] write overseer templates for $FORGE_PROJECT_NAME"
    exit 0
fi

PROJECT_SLUG="${FORGE_PROJECT_NAME}"
PROJECT_DIR="$FORGE_PROJECT_PATH"
WL_FILE="$PROJECT_DIR/.wl"

parse_wl_field() {
    local field="$1"
    grep -E "^${field}=" "$WL_FILE" 2>/dev/null | head -1 | sed 's/^[^=]*=//' | tr -d '"' | xargs
}

BUILD_CMD=$(parse_wl_field "build")
RUN_CMD=$(parse_wl_field "run")
TEST_CMD=$(parse_wl_field "test")
CHECK_CMD=$(parse_wl_field "check")

BUILD_CMD="${BUILD_CMD:-nix build}"
RUN_CMD="${RUN_CMD:-nix run}"
TEST_CMD="${TEST_CMD:-nix flake check}"
CHECK_CMD="${CHECK_CMD:-nix flake check}"

# Escape string for Lua
escape_lua() {
    printf '%s' "$1" | sed 's/["\\]/\\&/g'
}

TEMPLATE_DIR="${HOME}/.local/share/nvim/site/lua/overseer/template/forge"
mkdir -p "$TEMPLATE_DIR"

write_template() {
    local task_name="$1"
    local cmd="$2"
    local tag="$3"
    local desc="$4"
    local escaped=$(escape_lua "$cmd")

    cat > "${TEMPLATE_DIR}/${PROJECT_SLUG}_${task_name}.lua" << LUAEOF
return {
  name = "${PROJECT_SLUG}:${task_name}",
  builder = function()
    return {
      cmd = { "bash", "-c", "${escaped}" },
      cwd = "${PROJECT_DIR}",
      components = { "default" },
    }
  end,
  tags = { overseer.TAG.${tag} },
  desc = "${desc}",
}
LUAEOF
}

write_template "build" "$BUILD_CMD" "BUILD" "Build ${PROJECT_SLUG}"
write_template "run"   "$RUN_CMD"   "RUN"  "Run ${PROJECT_SLUG}"
write_template "check" "$CHECK_CMD" "TEST" "Check ${PROJECT_SLUG}"

echo "overseer: registered ${PROJECT_SLUG} (build/run/check)"