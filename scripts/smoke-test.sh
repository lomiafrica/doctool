#!/usr/bin/env bash
# Local smoke test for doctool — mirrors CI steps without full docs:drift on production corpus.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCTOOL_DIR="$ROOT/apps/doctool"
FIXTURE="$DOCTOOL_DIR/crates/doctool-core/tests/fixtures/mini-monorepo"

echo "==> cargo test (workspace)"
cd "$DOCTOOL_DIR"
cargo test --workspace

echo "==> dt scan on fixture"
cargo run -q -p doctool-cli -- \
  --config "$FIXTURE/doctool.config.toml" \
  --root "$FIXTURE" \
  --json scan | head -c 200
echo ""

echo "==> dt drift on fixture (expect failure)"
set +e
cargo run -q -p doctool-cli -- \
  --config "$FIXTURE/doctool.config.toml" \
  --root "$FIXTURE" \
  --skip-ts drift
DRIFT_EXIT=$?
set -e
if [[ "$DRIFT_EXIT" -eq 0 ]]; then
  echo "ERROR: expected drift to fail on fixture" >&2
  exit 1
fi
echo "drift correctly exited $DRIFT_EXIT"

echo "==> doctool build --release"
cargo build --release -p doctool-cli

echo "OK: doctool smoke tests passed"
