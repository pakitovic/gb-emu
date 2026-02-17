#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$ROOT_DIR/roms/gekkio's_test_roms"
VERSION="${GEKKIO_VERSION:-mts-20240926-1737-443f6e1}"
ZIP_URL="https://gekkio.fi/files/mooneye-test-suite/$VERSION/$VERSION.zip"

required_files="
acceptance/timer/div_write.gb
acceptance/timer/tim00.gb
acceptance/timer/tim00_div_trigger.gb
acceptance/timer/tim01.gb
acceptance/timer/tim01_div_trigger.gb
acceptance/timer/tim10.gb
acceptance/timer/tim10_div_trigger.gb
acceptance/timer/tim11.gb
acceptance/timer/tim11_div_trigger.gb
acceptance/timer/tima_reload.gb
acceptance/timer/rapid_toggle.gb
acceptance/timer/tima_write_reloading.gb
acceptance/timer/tma_write_reloading.gb
"

all_present=1
for rel in $required_files; do
  if [ ! -f "$DEST_DIR/$rel" ]; then
    all_present=0
    break
  fi
done

if [ "$all_present" -eq 1 ]; then
  echo "Gekkio ROMs already present in $DEST_DIR"
  exit 0
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

cd "$tmp_dir"
curl -fsSL -o mooneye.zip "$ZIP_URL"
unzip -q mooneye.zip

mkdir -p "$DEST_DIR"
cp -R "$tmp_dir/$VERSION/acceptance" "$DEST_DIR/"

echo "Fetched Gekkio ROMs into $DEST_DIR"
