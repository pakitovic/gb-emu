#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TIMEOUT_SECS="${TIMEOUT_SECS:-60}"
TEST_NAME="${TEST_NAME:-memory::tests::dma_scheduler_debug_guard_preserves_progress_under_stress}"

cd "$ROOT_DIR"

start_epoch="$(date +%s)"
output="$(
  perl -e "alarm $TIMEOUT_SECS; exec @ARGV" \
    cargo test --locked -p gb-emu --lib "$TEST_NAME" -- --exact 2>&1 || true
)"
end_epoch="$(date +%s)"
elapsed_secs=$((end_epoch - start_epoch))

if printf "%s" "$output" | rg -q "test result: ok"; then
  echo "dma guard | PASS | elapsed=${elapsed_secs}s | timeout=${TIMEOUT_SECS}s"
  exit 0
fi

if printf "%s" "$output" | rg -q "Alarm clock"; then
  echo "dma guard | TIMEOUT | elapsed=${elapsed_secs}s | timeout=${TIMEOUT_SECS}s"
  exit 1
fi

echo "dma guard | FAIL | elapsed=${elapsed_secs}s"
printf "%s\n" "$output"
exit 1
