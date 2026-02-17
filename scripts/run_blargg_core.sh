#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

ROM_ROOT="$ROOT_DIR/roms/blargg's_test_roms/cpu_instrs" "$ROOT_DIR/scripts/run_blargg_suite.sh"
ROM_ROOT="$ROOT_DIR/roms/blargg's_test_roms/instr_timing" "$ROOT_DIR/scripts/run_blargg_suite.sh"
