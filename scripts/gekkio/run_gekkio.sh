#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
ROM_ROOT="${ROM_ROOT:-}"
if [ -z "$ROM_ROOT" ]; then
  ROM_ROOT="$ROOT_DIR/roms/gekkio's_test_roms"
fi
BIN="$ROOT_DIR/target/debug/gb-emu"
MAX_STEPS="${MAX_STEPS:-120000000}"
TIMEOUT_SECS="${TIMEOUT_SECS:-30}"
GEKKIO_SUITE="${GEKKIO_SUITE:-all}"
GB_MODEL="${GB_MODEL:-dmg}"
ROM_LIST_FILE="$ROOT_DIR/scripts/gekkio/rom.txt"
BOOT_MODELS_LIST_FILE="$ROOT_DIR/scripts/gekkio/roms_boot_models.txt"

list_roms() {
  sed -e 's/\r$//' -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' "$@"
}

list_boot_model_rows() {
  awk '
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    NF != 2 {
      printf "Invalid boot model entry (expected: <model> <rom>): %s\n", $0 > "/dev/stderr"
      exit 1
    }
    $1 !~ /^(dmg0|dmg|mgb|sgb|sgb2)$/ {
      printf "Invalid boot model name: %s\n", $1 > "/dev/stderr"
      exit 1
    }
    { printf "%s|%s\n", $1, $2 }
  ' "$1"
}

if [ ! -d "$ROM_ROOT" ]; then
  echo "Gekkio ROM directory not found: $ROM_ROOT"
  exit 1
fi

run_mode=""
summary_model=""
suite_items=""

case "$GEKKIO_SUITE" in
  all | core)
    run_mode="single_model"
    summary_model="$GB_MODEL"
    suite_items="$(list_roms "$ROM_LIST_FILE")"
    ;;
  boot_models)
    run_mode="matrix"
    summary_model="matrix"
    suite_items="$(list_boot_model_rows "$BOOT_MODELS_LIST_FILE")"
    ;;
  *)
    echo "Unknown GEKKIO_SUITE: $GEKKIO_SUITE (expected: all|core|boot_models)"
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

run_case() {
  case_model="$1"
  rel="$2"

  total=$((total + 1))
  rom="$ROM_ROOT/$rel"
  if [ ! -f "$rom" ]; then
    echo "$case_model | $rel | MISSING_FILE"
    missing=$((missing + 1))
    return
  fi

  output="$(perl -e "alarm $TIMEOUT_SECS; exec @ARGV" "$BIN" --mooneye --model "$case_model" --max-steps "$MAX_STEPS" "$rom" 2>&1 || true)"

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

  echo "$case_model | $rel | $status"
}

if [ "$run_mode" = "single_model" ]; then
  for rel in $suite_items; do
    run_case "$GB_MODEL" "$rel"
  done
else
  for item in $suite_items; do
    case_model="${item%%|*}"
    rel="${item#*|}"
    run_case "$case_model" "$rel"
  done
fi

echo "----"
echo "MODEL=$summary_model SUITE=$GEKKIO_SUITE TOTAL=$total PASS=$pass FAIL=$fail TIMEOUT=$timeout MISSING=$missing UNKNOWN=$unknown"

if [ "$fail" -ne 0 ] || [ "$timeout" -ne 0 ] || [ "$missing" -ne 0 ] || [ "$unknown" -ne 0 ]; then
  exit 1
fi
