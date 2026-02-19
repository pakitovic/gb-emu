#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
ROM_ROOT="${ROM_ROOT:-}"
if [ -z "$ROM_ROOT" ]; then
  ROM_ROOT="$ROOT_DIR/roms/blargg's_test_roms"
fi
BIN="$ROOT_DIR/target/debug/gb-emu"
MAX_STEPS="${MAX_STEPS:-120000000}"
TIMEOUT_SECS="${TIMEOUT_SECS:-120}"
GB_MODEL="${GB_MODEL:-dmg}"
ROM_LIST_FILE="$ROOT_DIR/scripts/blargg/rom.txt"

list_roms() {
  sed -e 's/\r$//' -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' "$@"
}

if [ ! -d "$ROM_ROOT" ]; then
  echo "ROM directory not found: $ROM_ROOT"
  exit 1
fi

suite_list="$(mktemp)"
cleanup() {
  rm -f "$suite_list"
}
trap cleanup EXIT

list_roms "$ROM_LIST_FILE" > "$suite_list"

cd "$ROOT_DIR"
cargo build >/dev/null

pass=0
fail=0
missing_opcode=0
missing_file=0
timeout=0
unsupported=0
unknown=0
total=0

echo "Running Blargg suite: all"

while IFS= read -r rel; do
  total=$((total + 1))
  rom="$ROM_ROOT/$rel"
  if [ ! -f "$rom" ]; then
    printf "%s | MISSING_FILE\n" "$rel"
    missing_file=$((missing_file + 1))
    continue
  fi

  output="$(perl -e "alarm $TIMEOUT_SECS; exec @ARGV" "$BIN" --blargg --model "$GB_MODEL" --max-steps "$MAX_STEPS" "$rom" 2>&1 || true)"

  if printf "%s" "$output" | rg -q "Blargg result: Passed"; then
    status="PASS"
    pass=$((pass + 1))
  elif printf "%s" "$output" | rg -q "Blargg result: Failed"; then
    status="FAIL"
    fail=$((fail + 1))
  elif printf "%s" "$output" | rg -q "(^|[[:space:]])Passed([[:space:]]|$)"; then
    status="PASS"
    pass=$((pass + 1))
  elif printf "%s" "$output" | rg -q "(^|[[:space:]])Failed([[:space:]]|$)"; then
    status="FAIL"
    fail=$((fail + 1))
  elif printf "%s" "$output" | rg -q "Opcode not implemented: "; then
    op="$(printf "%s" "$output" | rg -o "Opcode not implemented: [0-9A-F]{2}" | tail -n1 | awk '{print $4}')"
    status="MISSING_OPCODE:$op"
    missing_opcode=$((missing_opcode + 1))
  elif printf "%s" "$output" | rg -q "Unsupported cartridge type|Unsupported ROM size code|Unsupported ROM file length"; then
    status="UNSUPPORTED"
    unsupported=$((unsupported + 1))
  elif printf "%s" "$output" | rg -q "did not finish within max steps|Alarm clock"; then
    status="TIMEOUT"
    timeout=$((timeout + 1))
  else
    status="UNKNOWN"
    unknown=$((unknown + 1))
  fi

  printf "%s | %s\n" "$rel" "$status"
done < "$suite_list"

echo "----"
echo "MODEL=$GB_MODEL SUITE=all TOTAL=$total PASS=$pass FAIL=$fail MISSING_OPCODE=$missing_opcode UNSUPPORTED=$unsupported TIMEOUT=$timeout MISSING_FILE=$missing_file UNKNOWN=$unknown"

if [ "$total" -eq 0 ]; then
  echo "No ROM entries configured in $ROM_LIST_FILE"
  exit 1
fi

if [ "$fail" -ne 0 ] || [ "$missing_opcode" -ne 0 ] || [ "$unsupported" -ne 0 ] || [ "$timeout" -ne 0 ] || [ "$missing_file" -ne 0 ] || [ "$unknown" -ne 0 ]; then
  exit 1
fi
