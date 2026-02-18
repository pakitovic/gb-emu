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
GEKKIO_SUITE="${GEKKIO_SUITE:-core}"
CORE_LIST_FILE="$ROOT_DIR/scripts/gekkio_roms_core.txt"
INCREMENTAL_LIST_FILE="$ROOT_DIR/scripts/gekkio_roms_incremental.txt"

list_roms() {
  sed -e 's/\r$//' -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' "$@"
}

if [ ! -d "$ROM_ROOT" ]; then
  echo "Gekkio ROM directory not found: $ROM_ROOT"
  exit 1
fi

case "$GEKKIO_SUITE" in
  core)
    roms="$(list_roms "$CORE_LIST_FILE")"
    ;;
  incremental)
    roms="$(list_roms "$CORE_LIST_FILE" "$INCREMENTAL_LIST_FILE")"
    ;;
  *)
    echo "Unknown GEKKIO_SUITE: $GEKKIO_SUITE (expected: core|incremental)"
    exit 1
    ;;
esac

cd "$ROOT_DIR"
cargo build >/dev/null

pass=0
fail=0
timeout=0
missing=0
unknown=0
total=0

echo "Running Gekkio suite: $GEKKIO_SUITE"

for rel in $roms; do
  total=$((total + 1))
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
echo "SUITE=$GEKKIO_SUITE TOTAL=$total PASS=$pass FAIL=$fail TIMEOUT=$timeout MISSING=$missing UNKNOWN=$unknown"

if [ "$fail" -ne 0 ] || [ "$timeout" -ne 0 ] || [ "$missing" -ne 0 ] || [ "$unknown" -ne 0 ]; then
  exit 1
fi
