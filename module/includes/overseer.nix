{
  description = "Add overseer task runner integration";
  provides = [ "overseer" ];
  setup = ''
    set -e
    TEMPLATE_DIR="$HOME/.local/share/nvim/site/lua/overseer/template/forge"
    mkdir -p "$TEMPLATE_DIR"
    parse_wl_field() {
      grep -E "^$1=" "$FORGE_PROJECT_PATH/.wl" 2>/dev/null | head -1 | sed 's/^[^=]*=//' | tr -d '"' | xargs
    }
    BUILD_CMD=$(parse_wl_field "build")
    RUN_CMD=$(parse_wl_field "run")
    TEST_CMD=$(parse_wl_field "test")
    CHECK_CMD=$(parse_wl_field "check")

    BUILD_CMD="''${BUILD_CMD:-nix build}"
    RUN_CMD="''${RUN_CMD:-nix run}"
    TEST_CMD="''${TEST_CMD:-nix flake check}"
    CHECK_CMD="''${CHECK_CMD:-nix flake check}"

    escape_lua() { printf '%s' "$1" | sed 's/["\\]/\\&/g'; }
    write_template() {
      local task_name="$1" cmd="$2" tag="$3"
      local escaped=$(escape_lua "$cmd")
      cat > "''${TEMPLATE_DIR}/''${FORGE_PROJECT_NAME}_''${task_name}.lua" << LUAEOF
    return {
      name = "''${FORGE_PROJECT_NAME}:''${task_name}",
      builder = function()
        return {
          cmd = { "bash", "-c", "''${escaped}" },
          cwd = "''${FORGE_PROJECT_PATH}",
          components = { "default" },
        }
      end,
      tags = { overseer.TAG.''${tag} },
      desc = "''${FORGE_PROJECT_NAME}:''${task_name}",
    }
    LUAEOF
    }
    write_template "build" "$BUILD_CMD" "BUILD"
    write_template "run" "$RUN_CMD" "RUN"
    write_template "check" "$CHECK_CMD" "TEST"
  '';
}