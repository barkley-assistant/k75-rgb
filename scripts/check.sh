#!/usr/bin/env bash
# Full offline check gate for the K75 repo. No device access required.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt =="
cargo fmt --manifest-path tools/hidra_probe/Cargo.toml --check

echo "== cargo test =="
cargo test --manifest-path tools/hidra_probe/Cargo.toml

echo "== firmware-signature guard =="
python3 analysis/verify_matrix_path.py

echo "== markdown link check =="
python3 scripts/check_links.py

echo "== local-path sweep =="
if grep -rn --include="*.rs" --include="*.md" --include="*.py" \
    -E "/home/|/tmp/|\.hermes" . \
    --exclude-dir=.git --exclude-dir=target --exclude-dir=analysis/video-evidence-2026-09-26 \
    | grep -v "gitignore"; then
  echo "FAIL: local path references found"
  exit 1
fi
echo "all checks green"