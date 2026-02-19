#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
DEST_DIR="$ROOT_DIR/roms/blargg's_test_roms"
REPO_URL="https://github.com/retrio/gb-test-roms.git"
ROM_LIST_FILE="$ROOT_DIR/scripts/blargg/rom.txt"
LISTING_FILE="$DEST_DIR/.blargg_listing.txt"

list_roms() {
  sed -e 's/\r$//' -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' "$@"
}

write_listing_file() {
  mkdir -p "$DEST_DIR"
  list_roms "$ROM_LIST_FILE" > "$LISTING_FILE"
}

required_list="$(mktemp)"
tmp_dir=""
cleanup() {
  rm -f "$required_list"
  if [ -n "$tmp_dir" ] && [ -d "$tmp_dir" ]; then
    rm -rf "$tmp_dir"
  fi
}
trap cleanup EXIT

list_roms "$ROM_LIST_FILE" > "$required_list"

all_present=1
while IFS= read -r rel; do
  if [ ! -f "$DEST_DIR/$rel" ]; then
    all_present=0
    break
  fi
done < "$required_list"

if [ "$all_present" -eq 1 ]; then
  write_listing_file
  echo "Blargg ROMs already present in $DEST_DIR"
  exit 0
fi

tmp_dir="$(mktemp -d)"
git clone --depth 1 "$REPO_URL" "$tmp_dir/gb-test-roms"

rm -rf "$DEST_DIR"
mkdir -p "$DEST_DIR"

while IFS= read -r rel; do
  src="$tmp_dir/gb-test-roms/$rel"
  dst="$DEST_DIR/$rel"
  if [ ! -f "$src" ]; then
    echo "Missing ROM in downloaded suite: $rel"
    exit 1
  fi

  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
done < "$required_list"

write_listing_file

echo "Fetched Blargg ROMs into $DEST_DIR"
