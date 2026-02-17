#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$ROOT_DIR/roms/blargg's_test_roms"

if [ -d "$DEST_DIR/cpu_instrs" ] && [ -d "$DEST_DIR/instr_timing" ] && [ -d "$DEST_DIR/mem_timing" ]; then
  echo "Blargg ROMs already present in $DEST_DIR"
  exit 0
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

git clone --depth 1 https://github.com/retrio/gb-test-roms.git "$tmp_dir/gb-test-roms"

mkdir -p "$DEST_DIR"
cp -R "$tmp_dir/gb-test-roms/cpu_instrs" "$DEST_DIR/"
cp -R "$tmp_dir/gb-test-roms/instr_timing" "$DEST_DIR/"
cp -R "$tmp_dir/gb-test-roms/mem_timing" "$DEST_DIR/"
cp "$tmp_dir/gb-test-roms/halt_bug.gb" "$DEST_DIR/"

echo "Fetched Blargg ROMs into $DEST_DIR"
