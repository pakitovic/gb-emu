#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
ROM_ROOT="${ROM_ROOT:-}"
if [ -z "$ROM_ROOT" ]; then
  ROM_ROOT="$ROOT_DIR/roms/blargg's_test_roms"
fi

BIN="$ROOT_DIR/target/debug/gb-emu"
ROM_REL_PATH="${ROM_REL_PATH:-cpu_instrs/cpu_instrs.gb}"
MAX_STEPS="${MAX_STEPS:-120000000}"
TIMEOUT_SECS="${TIMEOUT_SECS:-120}"
GB_MODEL="${GB_MODEL:-dmg}"

rom_path="$ROM_ROOT/$ROM_REL_PATH"
if [ ! -f "$rom_path" ]; then
  echo "ROM file not found: $rom_path"
  exit 1
fi

cd "$ROOT_DIR"
cargo build >/dev/null

start_epoch="$(date +%s)"
output="$(perl -e "alarm $TIMEOUT_SECS; exec @ARGV" "$BIN" --blargg --model "$GB_MODEL" --max-steps "$MAX_STEPS" "$rom_path" 2>&1 || true)"
end_epoch="$(date +%s)"
elapsed_secs=$((end_epoch - start_epoch))

if printf "%s" "$output" | rg -q "Blargg result: Passed"; then
  echo "cpu_instrs guard | PASS | elapsed=${elapsed_secs}s | timeout=${TIMEOUT_SECS}s"
  exit 0
fi

if printf "%s" "$output" | rg -q "(^|[[:space:]])Passed([[:space:]]|$)"; then
  echo "cpu_instrs guard | PASS | elapsed=${elapsed_secs}s | timeout=${TIMEOUT_SECS}s"
  exit 0
fi

if printf "%s" "$output" | rg -q "did not finish within max steps|Alarm clock"; then
  echo "cpu_instrs guard | TIMEOUT | elapsed=${elapsed_secs}s | timeout=${TIMEOUT_SECS}s"
  exit 1
fi

if printf "%s" "$output" | rg -q "Blargg result: Failed|(^|[[:space:]])Failed([[:space:]]|$)"; then
  echo "cpu_instrs guard | FAIL | elapsed=${elapsed_secs}s"
  exit 1
fi

echo "cpu_instrs guard | UNKNOWN | elapsed=${elapsed_secs}s"
printf "%s\n" "$output"
exit 1
