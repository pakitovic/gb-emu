#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ROM_ROOT="${ROM_ROOT:-}"
if [ -z "$ROM_ROOT" ]; then
  ROM_ROOT="$ROOT_DIR/roms/gekkio's_test_roms"
fi
BIN="$ROOT_DIR/target/debug/gb-emu"
MAX_STEPS="${MAX_STEPS:-120000000}"
TIMEOUT_SECS="${TIMEOUT_SECS:-30}"

if [ ! -d "$ROM_ROOT" ]; then
  echo "Gekkio ROM directory not found, skipping smoke suite: $ROM_ROOT"
  exit 0
fi

cd "$ROOT_DIR"
cargo build >/dev/null

pass=0
fail=0
timeout=0
missing=0
unknown=0

roms="
acceptance/timer/div_write.gb
acceptance/timer/tim00.gb
acceptance/timer/tim01.gb
acceptance/timer/tim10.gb
acceptance/timer/tim11.gb
acceptance/timer/tima_reload.gb
"

for rel in $roms; do
  rom="$ROM_ROOT/$rel"
  if [ ! -f "$rom" ]; then
    echo "$rel | MISSING_FILE"
    missing=$((missing + 1))
    continue
  fi

  output="$(perl -e "alarm $TIMEOUT_SECS; exec @ARGV" "$BIN" --mooneye --max-steps "$MAX_STEPS" "$rom" 2>&1 || true)"

  if printf "%s" "$output" | rg -q "Mooneye result: Passed"; then
    status="PASS"
    pass=$((pass + 1))
  elif printf "%s" "$output" | rg -q "Mooneye result: Failed"; then
    status="FAIL"
    fail=$((fail + 1))
  elif printf "%s" "$output" | rg -q "did not finish within max steps"; then
    status="TIMEOUT"
    timeout=$((timeout + 1))
  else
    status="UNKNOWN"
    unknown=$((unknown + 1))
  fi

  echo "$rel | $status"
done

echo "----"
echo "PASS=$pass FAIL=$fail TIMEOUT=$timeout MISSING=$missing UNKNOWN=$unknown"

if [ "$fail" -ne 0 ] || [ "$timeout" -ne 0 ] || [ "$missing" -ne 0 ] || [ "$unknown" -ne 0 ]; then
  exit 1
fi
