#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== Running Rust test suite at $(date) ==="
cd "${ROOT}/src-tauri"
cargo test