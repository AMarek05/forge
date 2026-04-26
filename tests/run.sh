#!/usr/bin/env bash
# forge test suite runner
# Run all tests: ./tests/run.sh
# Run specific: ./tests/run.sh unit

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

run_unit() {
  echo "Running unit tests..."
  echo "NOTE: Unit tests (Rust) require cargo test"
  echo "Run manually: cargo test --manifest-path '$SCRIPT_DIR/../Cargo.toml'"
}

run_module() {
  echo "Running module tests..."
  echo "NOTE: Module tests are Nix eval queries in tests/module/queries.md"
  echo "Run manually: nix eval -f . homeManagerModules.default"
}

run_integration() {
  echo "Running integration tests..."
  bash "$SCRIPT_DIR/integration/suite.sh"
}

run_shell() {
  echo "Running shell/completion tests..."
  bash "$SCRIPT_DIR/shell/completion-tests.sh"
}

run_all() {
  run_module
  run_integration
  run_shell
}

case "${1:-all}" in
  unit)
    run_unit
    ;;
  module)
    run_module
    ;;
  integration)
    run_integration
    ;;
  shell)
    run_shell
    ;;
  all)
    run_all
    ;;
  *)
    echo "Usage: $0 {unit|module|integration|shell|all}"
    exit 1
    ;;
esac