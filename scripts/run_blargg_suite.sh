#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ROM_ROOT="${ROM_ROOT:-}"
if [ -z "$ROM_ROOT" ]; then
  ROM_ROOT="$ROOT_DIR/roms/blargg's_test_roms"
fi
BIN="$ROOT_DIR/target/debug/gb-emu"
MAX_STEPS="${MAX_STEPS:-120000000}"
TIMEOUT_SECS="${TIMEOUT_SECS:-30}"

if [ ! -d "$ROM_ROOT" ]; then
  echo "ROM directory not found: $ROM_ROOT"
  exit 1
fi

cd "$ROOT_DIR"
cargo build >/dev/null

pass=0
fail=0
missing=0
timeout=0
unknown=0

found=0
rom_list="$(mktemp)"
trap 'rm -f "$rom_list"' EXIT
find "$ROM_ROOT" -type f -name "*.gb" | sort > "$rom_list"

while IFS= read -r rom; do
  found=1
  output="$(perl -e "alarm $TIMEOUT_SECS; exec @ARGV" "$BIN" --blargg --max-steps "$MAX_STEPS" "$rom" 2>&1 || true)"

  if printf "%s" "$output" | rg -q "Blargg result: Passed"; then
    status="PASS"
    pass=$((pass + 1))
  elif printf "%s" "$output" | rg -q "Blargg result: Failed"; then
    status="FAIL"
    fail=$((fail + 1))
  elif printf "%s" "$output" | rg -q "Opcode not implemented: "; then
    op="$(printf "%s" "$output" | rg -o "Opcode not implemented: [0-9A-F]{2}" | tail -n1 | awk '{print $4}')"
    status="MISSING_OPCODE:$op"
    missing=$((missing + 1))
  elif printf "%s" "$output" | rg -q "did not finish within max steps"; then
    status="TIMEOUT"
    timeout=$((timeout + 1))
  else
    status="UNKNOWN"
    unknown=$((unknown + 1))
  fi

  rel="${rom#$ROM_ROOT/}"
  printf "%s | %s\n" "$rel" "$status"
done < "$rom_list"

echo "----"
echo "PASS=$pass FAIL=$fail MISSING=$missing TIMEOUT=$timeout UNKNOWN=$unknown"

if [ "$found" -eq 0 ]; then
  echo "No .gb ROM files found under $ROM_ROOT"
  exit 1
fi

if [ "$fail" -ne 0 ] || [ "$missing" -ne 0 ] || [ "$timeout" -ne 0 ] || [ "$unknown" -ne 0 ]; then
  exit 1
fi
