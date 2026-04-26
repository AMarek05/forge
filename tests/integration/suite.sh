#!/usr/bin/env bash
# forge integration tests — require a real forge binary
set -e

FORGE="${FORGE:-$(nix build .#forge --print-out-paths --quiet 2>/dev/null)/bin/forge}"
TEST_DIR="${TMPDIR:-/tmp}/forge-test-$$"
SYNC_BASE="$TEST_DIR/sync"

export FORGE_SYNC_BASE="$SYNC_BASE"
export FORGE_BASE="$TEST_DIR/.forge"

cleanup() { rm -rf "$TEST_DIR"; }
trap cleanup EXIT

echo "=== Integration Tests ==="

# Test: dry-run create
echo "[TEST] forge create --dry-run"
$FORGE create test-dry --lang rust --no-open --dry-run >/dev/null
echo "  PASS: dry-run succeeds"

# Test: list with no projects
echo "[TEST] forge list (empty)"
count=$($FORGE list | grep -c "no projects found" || true)
if [ "$count" -eq "1" ]; then
  echo "  PASS: empty list works"
else
  echo "  FAIL: expected 'no projects found'"
  exit 1
fi

# Test: create rust project
echo "[TEST] forge create rust"
$FORGE create test-rust --lang rust --no-open >/dev/null
if [ -f "$SYNC_BASE/Code/Rust/test-rust/.wl" ]; then
  echo "  PASS: .wl created"
else
  echo "  FAIL: .wl not found"
  exit 1
fi

# Test: create python project
echo "[TEST] forge create python"
$FORGE create test-python --lang python --no-open >/dev/null
if [ -f "$SYNC_BASE/Code/Python/test-python/.wl" ]; then
  echo "  PASS: python .wl created"
else
  echo "  FAIL: python .wl not found"
  exit 1
fi

# Test: list shows projects
echo "[TEST] forge list (with projects)"
names=$($FORGE list | grep -E "test-rust|test-python" || true)
if [ -n "$names" ]; then
  echo "  PASS: list shows projects"
else
  echo "  FAIL: list is empty"
  exit 1
fi

# Test: forge cd --print
echo "[TEST] forge cd --print"
path=$($FORGE cd test-rust --print)
if [ "$path" = "$SYNC_BASE/Code/Rust/test-rust" ]; then
  echo "  PASS: cd --print correct"
else
  echo "  FAIL: cd returned '$path'"
  exit 1
fi

# Test: remove
echo "[TEST] forge remove"
$FORGE remove test-python >/dev/null
found=$($FORGE list | grep -c "test-python" || true)
if [ "$found" -eq "0" ]; then
  echo "  PASS: remove works"
else
  echo "  FAIL: project still in list"
  exit 1
fi

# Test: create with includes
echo "[TEST] forge create with --include git"
$FORGE create test-with-git --lang rust --no-open --include git >/dev/null
wl_content=$(cat "$SYNC_BASE/Code/Rust/test-with-git/.wl")
if echo "$wl_content" | grep -q "git"; then
  echo "  PASS: includes in .wl"
else
  echo "  FAIL: includes not written"
  exit 1
fi

# Test: overseer --regen (no projects from test but should not error)
echo "[TEST] forge overseer --regen (no projects)"
$FORGE overseer --regen >/dev/null 2>&1 && echo "  PASS: overseer regen no-ops gracefully" || {
  echo "  FAIL: overseer regen errored"
  exit 1
}

echo ""
echo "=== All integration tests passed ==="