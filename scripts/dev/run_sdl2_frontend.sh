#!/usr/bin/env sh
set -eu

show_help() {
  cat <<'EOF'
Usage: scripts/dev/run_sdl2_frontend.sh [options] [-- <rom_path> [model]]

Builds the SDL2 frontend with --locked and optional clean rebuild.
On macOS, when Homebrew SDL2 is detected, it exports linker/include/pkg-config
paths for this command invocation.

Options:
  --clean         Run `cargo clean -p gb-emu -p frontend-sdl2` before build (default).
  --no-clean      Skip clean step.
  --no-run        Build only, do not run even if ROM is provided.
  -h, --help      Show this help.

Examples:
  scripts/dev/run_sdl2_frontend.sh
  scripts/dev/run_sdl2_frontend.sh --no-clean
  scripts/dev/run_sdl2_frontend.sh -- path/to/game.gb
  scripts/dev/run_sdl2_frontend.sh -- path/to/game.gb mgb
EOF
}

clean=1
run_after_build=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --clean)
      clean=1
      ;;
    --no-clean)
      clean=0
      ;;
    --no-run)
      run_after_build=0
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      printf "Unknown option: %s\n\n" "$1" >&2
      show_help >&2
      exit 1
      ;;
    *)
      break
      ;;
  esac
  shift
done

rom_path="${1:-}"
model="${2:-}"
if [ "$#" -gt 2 ]; then
  printf "Expected optional ROM path and optional model only\n\n" >&2
  show_help >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"

prepend_env_path() {
  var_name="$1"
  add_path="$2"
  if [ ! -d "$add_path" ]; then
    return
  fi
  current_value="$(eval "printf '%s' \"\${$var_name:-}\"")"
  case ":$current_value:" in
    *":$add_path:"*) return ;;
  esac
  if [ -n "$current_value" ]; then
    eval "export $var_name=\"$add_path:$current_value\""
  else
    eval "export $var_name=\"$add_path\""
  fi
}

configure_homebrew_sdl2_env() {
  if [ "$(uname -s)" != "Darwin" ]; then
    return
  fi
  if ! command -v brew >/dev/null 2>&1; then
    return
  fi
  if ! brew_prefix="$(brew --prefix 2>/dev/null)"; then
    return
  fi

  sdl2_prefix=""
  if sdl2_prefix_candidate="$(brew --prefix sdl2 2>/dev/null)"; then
    sdl2_prefix="$sdl2_prefix_candidate"
  fi

  prepend_env_path PKG_CONFIG_PATH "$brew_prefix/lib/pkgconfig"
  prepend_env_path LIBRARY_PATH "$brew_prefix/lib"
  prepend_env_path CPATH "$brew_prefix/include"
  prepend_env_path DYLD_FALLBACK_LIBRARY_PATH "$brew_prefix/lib"

  if [ -n "$sdl2_prefix" ]; then
    prepend_env_path PKG_CONFIG_PATH "$sdl2_prefix/lib/pkgconfig"
    prepend_env_path LIBRARY_PATH "$sdl2_prefix/lib"
    prepend_env_path CPATH "$sdl2_prefix/include"
    prepend_env_path DYLD_FALLBACK_LIBRARY_PATH "$sdl2_prefix/lib"
  fi
}

configure_homebrew_sdl2_env

cd "$ROOT_DIR"

if [ "$clean" -eq 1 ]; then
  printf "Cleaning gb-emu + frontend-sdl2 artifacts...\n"
  cargo clean -p gb-emu -p frontend-sdl2
fi

printf "Building SDL2 frontend...\n"
cargo build --locked -p frontend-sdl2 --bin frontend-sdl2

if [ "$run_after_build" -eq 0 ]; then
  printf "Build completed (run skipped).\n"
  exit 0
fi

if [ -z "$rom_path" ]; then
  printf "Build completed. Pass a ROM to run:\n"
  printf "  scripts/dev/run_sdl2_frontend.sh -- <path_to_rom.gb> [dmg0|dmg|mgb|sgb|sgb2]\n"
  exit 0
fi

if [ ! -f "$rom_path" ]; then
  printf "ROM file not found: %s\n" "$rom_path" >&2
  exit 1
fi

printf "Launching SDL2 frontend...\n"
if [ -n "$model" ]; then
  exec "$ROOT_DIR/target/debug/frontend-sdl2" "$rom_path" "$model"
fi
exec "$ROOT_DIR/target/debug/frontend-sdl2" "$rom_path"
