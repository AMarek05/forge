#!/usr/bin/env bash
# forge integration tests — require a real forge binary
set -e

FORGE="${FORGE:-$(nix build .#forge --print-out-paths --quiet 2>/dev/null)/bin/forge}"
TEST_DIR="${TMPDIR:-/tmp}/forge-test-$$"
SYNC_BASE="$TEST_DIR/sync"

export HOME="$TEST_DIR"
mkdir -p "$HOME/.forge"
cat > "$HOME/.forge/config.sh" << 'EOF'
export FORGE_SYNC_BASE="$SYNC_BASE"
export FORGE_BASE="$TEST_DIR/.forge"
export FORGE_LANG_DIR="$(dirname "$FORGE")/../share/forge/languages"
EOF

cat > "$HOME/.forge/test_env" << EOF
FORGE_SYNC_BASE="$SYNC_BASE"
FORGE_BASE="$TEST_DIR/.forge"
FORGE_LANG_DIR="$(dirname "$FORGE")/../share/forge/languages"
EOF
set -a; source "$HOME/.forge/test_env"; set +a

cleanup() { rm -rf "$TEST_DIR"; }
trap cleanup EXIT

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
PASSED=0
FAILED=0

pass() { printf "${GREEN}[PASS]${NC} %s\n" "$1"; PASSED=$((PASSED + 1)); }
fail() { printf "${RED}[FAIL]${NC} %s\n" "$1"; FAILED=$((FAILED + 1)); exit 1; }
info() { printf "${YELLOW}[INFO]${NC} %s\n" "$1"; }

echo "=== Integration Tests ==="
echo ""

# Test: dry-run create
info "forge create --dry-run"
$FORGE create test-dry --lang rust --no-open --dry-run >/dev/null && pass "dry-run succeeds" || fail "dry-run failed"

# Test: list with no projects
info "forge list (empty index)"
output=$($FORGE list 2>&1) || true
info "list output: $output"
if echo "$output" | grep -qi "no projects"; then
  pass "empty list shows 'no projects'"
else
  fail "expected 'no projects' in output, got: $output"
fi

# Test: create rust project
info "forge create test-rust --lang rust --no-open"
$FORGE create test-rust --lang rust --no-open || fail "create rust failed"
wl_path="$SYNC_BASE/Code/Rust/test-rust/.wl"
if [ -f "$wl_path" ]; then
  pass ".wl created at $wl_path"
else
  fail ".wl not found at $wl_path (contents of Code/Rust: $(ls -la "$SYNC_BASE/Code/Rust/" 2>/dev/null || echo '(dir missing)'))"
fi

# Verify .wl content
info "checking .wl content"
wl_content=$(cat "$wl_path")
info ".wl content: $wl_content"
if echo "$wl_content" | grep -q "name="; then
  pass ".wl contains 'name='"
else
  fail ".wl missing 'name=' field"
fi

# Test: create python project
info "forge create test-python --lang python --no-open"
$FORGE create test-python --lang python --no-open || fail "create python failed"
py_wl="$SYNC_BASE/Code/Python/test-python/.wl"
if [ -f "$py_wl" ]; then
  pass "python .wl created"
else
  fail "python .wl not found"
fi

# Test: list shows projects
info "forge list (with projects)"
list_output=$($FORGE list 2>&1) || true
info "list output: $list_output"
if echo "$list_output" | grep -q "test-rust"; then
  pass "list shows test-rust"
else
  fail "test-rust not in list output"
fi
if echo "$list_output" | grep -q "test-python"; then
  pass "list shows test-python"
else
  fail "test-python not in list output"
fi

# Test: forge cd --print
info "forge cd test-rust --print"
cd_output=$($FORGE cd test-rust --print 2>&1) || fail "cd failed"
expected="$SYNC_BASE/Code/Rust/test-rust"
info "cd output: $cd_output"
info "expected: $expected"
if [ "$cd_output" = "$expected" ]; then
  pass "cd --print correct"
else
  fail "cd returned '$cd_output', expected '$expected'"
fi

# Test: remove
info "forge remove test-python"
$FORGE remove test-python || fail "remove failed"
list_after=$($FORGE list 2>&1) || true
info "list after remove: $list_after"
if echo "$list_after" | grep -q "test-python"; then
  fail "test-python still in list after remove"
else
  pass "remove works"
fi

# Test: create with includes
info "forge create test-with-git --lang rust --no-open --include git"
$FORGE create test-with-git --lang rust --no-open --include git || fail "create with git include failed"
git_wl="$SYNC_BASE/Code/Rust/test-with-git/.wl"
wl_content=$(cat "$git_wl")
info ".wl content: $wl_content"
if echo "$wl_content" | grep -qi "git"; then
  pass "includes field contains git"
else
  fail "includes not in .wl"
fi

# Test: overseer --regen (no projects from test but should not error)
info "forge overseer --regen"
$FORGE overseer --regen >/dev/null 2>&1 && pass "overseer regen no-ops gracefully" || fail "overseer regen errored"

echo ""
echo "=== Summary: $PASSED passed, $FAILED failed ==="
[ "$FAILED" -eq 0 ] || exit 1
