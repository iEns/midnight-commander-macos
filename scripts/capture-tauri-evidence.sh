#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUNDLE="${BUNDLE:-${ROOT}/src-tauri/target/release/bundle/macos/Midnight Commander.app}"
PLIST="${BUNDLE}/Contents/Info.plist"
EXECUTABLE="${BUNDLE}/Contents/MacOS/midnight-commander"
SCRATCH="${SCRATCH:-/var/folders/bb/294bcl1j3rg18yc5svx32ns40000gp/T/grok-goal-4bd88946786a/implementer}"

mkdir -p "${SCRATCH}"

{
  echo "=== bundle-structure.log ==="
  echo "Date: $(date)"
  echo "Bundle: ${BUNDLE}"
  echo
  echo "--- plutil -lint ---"
  plutil -lint "${PLIST}"
  echo
  echo "--- executable ---"
  ls -la "${EXECUTABLE}"
  echo
  echo "--- CFBundleIdentifier ---"
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${PLIST}"
  echo
  echo "--- mdimport + mdls (quoted bundle path) ---"
  mdimport "${BUNDLE}" 2>&1 || true
  sleep 2
  mdls -name kMDItemKind -name kMDItemContentType "${BUNDLE}"
  echo
  echo "--- PlistBuddy CFBundlePackageType ---"
  /usr/libexec/PlistBuddy -c 'Print :CFBundlePackageType' "${PLIST}"
} > "${SCRATCH}/bundle-structure.log" 2>&1

{
  echo "=== unit-tests.log ==="
  echo "Date: $(date)"
  cd "${ROOT}/src-tauri"
  cargo test 2>&1
} > "${SCRATCH}/unit-tests.log" 2>&1

{
  echo "=== pty-tests.log ==="
  echo "Date: $(date)"
  cd "${ROOT}/src-tauri"
  cargo test pty_registry 2>&1
} > "${SCRATCH}/pty-tests.log" 2>&1

{
  echo "=== evidence.log ==="
  echo "Date: $(date)"
  echo
  echo "--- file ---"
  file "${BUNDLE}"
  echo
  echo "--- codesign ---"
  codesign -dv "${BUNDLE}" 2>&1 || echo "(not signed)"
  echo
  echo "--- build target excerpt ---"
  rg -n "Bundling Midnight Commander.app|Finished 1 bundle" "${SCRATCH}/build.log" 2>/dev/null || true
} > "${SCRATCH}/evidence.log" 2>&1

echo "Evidence captured under ${SCRATCH}"