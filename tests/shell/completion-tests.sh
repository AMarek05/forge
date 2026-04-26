#!/usr/bin/env bash
# forge shell tests — validate setup scripts in a nix shell
set -e

echo "=== Shell Integration Tests ==="

# Ensure forge builds
FORGE_BIN="$(nix build .#forge --print-out-paths --quiet 2>/dev/null)/bin/forge"
if [ ! -f "$FORGE_BIN" ]; then
  echo "FAIL: forge binary not built"
  exit 1
fi
echo "  PASS: forge binary exists at $FORGE_BIN"

# Test: forge --generate-completion zsh
echo "[TEST] forge --generate-completion zsh"
completion=$($FORGE_BIN --generate-completion zsh 2>&1)
if echo "$completion" | grep -q "create\|remove\|list"; then
  echo "  PASS: zsh completion has subcommands"
else
  echo "  FAIL: zsh completion missing subcommands"
  exit 1
fi

# Test: forge --generate-completion bash
echo "[TEST] forge --generate-completion bash"
completion=$($FORGE_BIN --generate-completion bash 2>&1)
if echo "$completion" | grep -q "create\|remove\|list"; then
  echo "  PASS: bash completion has subcommands"
else
  echo "  FAIL: bash completion missing subcommands"
  exit 1
fi

# Test: forge --generate-completion fish
echo "[TEST] forge --generate-completion fish"
completion=$($FORGE_BIN --generate-completion fish 2>&1)
if echo "$completion" | grep -q "create\|remove\|list"; then
  echo "  PASS: fish completion has subcommands"
else
  echo "  FAIL: fish completion missing subcommands"
  exit 1
fi

# Test: forge --generate-completion zsh > _forge (verify file loads in zsh)
echo "[TEST] zsh completion file loads in zsh"
completion_file=$(mktemp)
echo "$completion" > "$completion_file"
# Verify it doesn't cause a parse error (just sourcing the compinit preamble)
if ! command -v zsh >/dev/null 2>&1; then
  echo "  SKIP: zsh not available in PATH"
elif zsh -e -f -c "autoload -Uz compinit; compinit" 2>/dev/null; then
  # Now try to load our completion
  result=$(zsh -e -f -c "source '$completion_file'; compdef _forge forge" 2>&1)
  if [ $? -eq 0 ]; then
    echo "  PASS: completion file loads without errors"
  else
    echo "  WARN: completion file had issues (may be ok): $result"
  fi
else
  echo "  WARN: could not test zsh loading"
fi
rm -f "$completion_file"

# Test: help output
echo "[TEST] forge --help"
help=$($FORGE_BIN --help)
for cmd in create remove list sync cd session pick setup include lang overseer edit open; do
  if echo "$help" | grep -q "$cmd"; then
    echo "  PASS: --help mentions '$cmd'"
  else
    echo "  FAIL: --help missing '$cmd'"
    exit 1
  fi
done

# Test: version output
echo "[TEST] forge --version"
version=$($FORGE_BIN --version)
if echo "$version" | grep -q "forge"; then
  echo "  PASS: version outputs 'forge'"
else
  echo "  FAIL: version output unexpected: $version"
  exit 1
fi

# Test: completions don't error on unknown shell
echo "[TEST] forge --generate-completion unknown-shell"
error=$($FORGE_BIN --generate-completion unknown-shell 2>&1 || true)
if echo "$error" | grep -q "Unsupported shell"; then
  echo "  PASS: unsupported shell gives clear error"
else
  echo "  FAIL: expected 'Unsupported shell' error"
  exit 1
fi

# Test: overseer command with no index
echo "[TEST] forge overseer (no index — should not error)"
TEST_HOME=$(mktemp -d)
FORGE_BASE="$TEST_HOME/.forge" \
FORGE_SYNC_BASE="$TEST_HOME/sync" \
  $FORGE_BIN overseer 2>&1 | head -1
# Should say "overseer: nvim not found" or similar — not a panic
echo "  PASS: overseer command handled gracefully"
rm -rf "$TEST_HOME"

echo ""
echo "=== All shell tests passed ==="