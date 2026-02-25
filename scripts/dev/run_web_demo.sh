#!/usr/bin/env sh
set -eu

show_help() {
  cat <<'EOF'
Usage: scripts/dev/run_web_demo.sh [options]

Builds the Rust/WASM frontend package (`frontends/wasm`) from a clean pkg output,
syncs generated pkg
artifacts into web/pkg,
and optionally serves the project root so the demo is available at
http://localhost:<port>/web/

Options:
  --port <port>  HTTP port for python3 http.server (default: 8080).
  --no-serve     Build and sync artifacts only (do not start server).
  -h, --help     Show this help.
EOF
}

port="8080"
serve=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --port)
      shift
      if [ "$#" -eq 0 ]; then
        printf "Missing value for --port\n" >&2
        exit 1
      fi
      port="$1"
      ;;
    --no-serve)
      serve=0
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    *)
      printf "Unknown option: %s\n\n" "$1" >&2
      show_help >&2
      exit 1
      ;;
  esac
  shift
done

require_cmd() {
  cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf "Missing required command: %s\n" "$cmd" >&2
    exit 1
  fi
}

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PKG_SOURCE_DIR="$ROOT_DIR/frontends/wasm/pkg"
PKG_DEST_DIR="$ROOT_DIR/web/pkg"

require_cmd wasm-pack
if [ "$serve" -eq 1 ]; then
  require_cmd python3
fi

printf "Building wasm frontend artifacts...\n"
cd "$ROOT_DIR"
rm -rf "$PKG_SOURCE_DIR" "$PKG_DEST_DIR"
wasm-pack build frontends/wasm --target web --out-name gb_emu

if [ ! -d "$PKG_SOURCE_DIR" ]; then
  printf "Expected wasm-pack output directory not found: %s\n" "$PKG_SOURCE_DIR" >&2
  exit 1
fi

rm -rf "$PKG_DEST_DIR"
mv "$PKG_SOURCE_DIR" "$PKG_DEST_DIR"

printf "Synced wasm artifacts to %s\n" "$PKG_DEST_DIR"
printf "# Open http://localhost:%s/web/\n" "$port"

if [ "$serve" -eq 1 ]; then
  python3 -m http.server "$port"
fi
