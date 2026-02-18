#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$ROOT_DIR/roms/gekkio's_test_roms"
VERSION="${GEKKIO_VERSION:-mts-20240926-1737-443f6e1}"
ZIP_URL="https://gekkio.fi/files/mooneye-test-suite/$VERSION/$VERSION.zip"
CORE_LIST_FILE="$ROOT_DIR/scripts/gekkio_roms_core.txt"
INCREMENTAL_LIST_FILE="$ROOT_DIR/scripts/gekkio_roms_incremental.txt"
BOOT_MODELS_LIST_FILE="$ROOT_DIR/scripts/gekkio_roms_boot_models.txt"

list_roms() {
  sed -e 's/\r$//' -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' "$@"
}

list_boot_model_roms() {
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
    { print $2 }
  ' "$1"
}

required_files="$(
  {
    list_roms "$CORE_LIST_FILE" "$INCREMENTAL_LIST_FILE"
    list_boot_model_roms "$BOOT_MODELS_LIST_FILE"
  } | sort -u
)"

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
for rel in $required_files; do
  src="$tmp_dir/$VERSION/$rel"
  dst="$DEST_DIR/$rel"
  if [ ! -f "$src" ]; then
    echo "Missing ROM in downloaded suite: $rel"
    exit 1
  fi

  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
done

echo "Fetched Gekkio ROMs into $DEST_DIR"
