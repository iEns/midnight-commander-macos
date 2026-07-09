#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUNDLE="${BUNDLE:-${ROOT}/src-tauri/target/release/bundle/macos/Midnight Commander.app}"
EXECUTABLE="${BUNDLE}/Contents/MacOS/midnight-commander"
SCRATCH="${SCRATCH:-/var/folders/bb/294bcl1j3rg18yc5svx32ns40000gp/T/grok-goal-4bd88946786a/implementer}"
LIVE_SNAPSHOT="${SCRATCH}/live-sessions.json"

mkdir -p "${SCRATCH}"
rm -f "${LIVE_SNAPSHOT}"

count_mc_processes() {
  pgrep -lf "libexec/bin/mc|midnight-commander.*/bin/mc" 2>/dev/null | wc -l | tr -d ' '
}

# Stop prior instances so launch evidence is deterministic.
pkill -f "${EXECUTABLE}" 2>/dev/null || true
sleep 1

open "${BUNDLE}"
OPEN1_EXIT=$?
sleep 8

{
  echo "open exit: ${OPEN1_EXIT}"
  echo "--- host process (Midnight Commander, not Terminal) ---"
  pgrep -lf "Midnight Commander.app/Contents/MacOS/midnight-commander" || true
  echo "--- mc processes after first launch ---"
  pgrep -lf "libexec/bin/mc|midnight-commander.*/bin/mc" || true
  echo "mc count: $(count_mc_processes)"
} > "${SCRATCH}/launch-1.log" 2>&1

open "${BUNDLE}"
OPEN2_EXIT=$?
sleep 3

{
  echo "second open exit: ${OPEN2_EXIT}"
  echo "--- mc processes after second open (same instance re-activated) ---"
  pgrep -lf "libexec/bin/mc|midnight-commander.*/bin/mc" || true
  echo "mc count: $(count_mc_processes)"
} >> "${SCRATCH}/launch-1.log" 2>&1

pkill -f "${EXECUTABLE}" 2>/dev/null || true
sleep 2

# Live-app verification: one process, shipped new_mc_window, real PTY registry.
MC_VERIFY_OUTPUT="${LIVE_SNAPSHOT}" "${EXECUTABLE}" --verify-live-multi-window >/dev/null 2>&1 &
LIVE_PID=$!

for _ in $(seq 1 40); do
  if [[ -f "${LIVE_SNAPSHOT}" ]]; then
    break
  fi
  sleep 1
done

sleep 2

{
  echo "=== launch-multi.log ==="
  echo "Date: $(date)"
  echo "live_app_pid: ${LIVE_PID}"
  echo "--- invoked shipped new_mc_window inside running app ---"
  echo "flag: --verify-live-multi-window"
  echo
  echo "--- live PTY registry snapshot ---"
  if [[ -f "${LIVE_SNAPSHOT}" ]]; then
    cat "${LIVE_SNAPSHOT}"
  else
    echo "ERROR: live snapshot missing at ${LIVE_SNAPSHOT}"
  fi
  echo
  echo "--- mc processes in live verification run ---"
  pgrep -lf "libexec/bin/mc|midnight-commander.*/bin/mc" || true
  echo "mc count: $(count_mc_processes)"
} > "${SCRATCH}/launch-multi.log" 2>&1

kill "${LIVE_PID}" 2>/dev/null || true
wait "${LIVE_PID}" 2>/dev/null || true

python3 - <<'PY' "${LIVE_SNAPSHOT}" "${SCRATCH}/launch-multi.log"
import json, sys
from pathlib import Path

snapshot_path = Path(sys.argv[1])
log_path = Path(sys.argv[2])

if not snapshot_path.exists():
    raise SystemExit("missing live snapshot")

data = json.loads(snapshot_path.read_text())
count = data.get("session_count", 0)
sessions = data.get("sessions", [])
ids = {s.get("session_id") for s in sessions}
labels = {s.get("window_label") for s in sessions}

with log_path.open("a") as fh:
    fh.write("\n--- assertion summary ---\n")
    fh.write(f"new_window_invoked: {data.get('new_window_invoked')}\n")
    fh.write(f"session_count: {count}\n")
    fh.write(f"distinct_session_ids: {len(ids)}\n")
    fh.write(f"window_labels: {sorted(labels)}\n")

if not data.get("new_window_invoked"):
    raise SystemExit("new_mc_window was not invoked in live app")
if count < 2 or len(ids) < 2:
    raise SystemExit(f"expected >=2 live sessions, got {count}")
PY

echo "Launch verification captured under ${SCRATCH}"