#!/usr/bin/env sh
set -eu

show_help() {
  cat <<'EOF'
Usage: scripts/dev/bootstrap.sh [options]

Checks local development dependencies used by this repository.

Options:
  --install-wasm-pack  Install wasm-pack via cargo when missing.
  --skip-sdl2          Skip SDL2 frontend dependency checks.
  --skip-web           Skip web frontend dependency checks.
  -h, --help           Show this help.
EOF
}

install_wasm_pack=0
check_sdl2=1
check_web=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-wasm-pack)
      install_wasm_pack=1
      ;;
    --skip-sdl2)
      check_sdl2=0
      ;;
    --skip-web)
      check_web=0
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

ready=1

mark_missing() {
  ready=0
}

check_required_cmd() {
  cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    printf "ok   required command: %s\n" "$cmd"
  else
    printf "miss required command: %s\n" "$cmd"
    mark_missing
  fi
}

print_sdl2_hint() {
  os_name="$(uname -s)"
  case "$os_name" in
    Darwin)
      printf "hint install SDL2 on macOS: brew install sdl2\n"
      ;;
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        printf "hint install SDL2 on Debian/Ubuntu: sudo apt-get update && sudo apt-get install -y libsdl2-dev\n"
      elif command -v dnf >/dev/null 2>&1; then
        printf "hint install SDL2 on Fedora: sudo dnf install -y SDL2-devel\n"
      elif command -v pacman >/dev/null 2>&1; then
        printf "hint install SDL2 on Arch: sudo pacman -S --needed sdl2\n"
      else
        printf "hint install SDL2 dev/runtime libraries using your distro package manager.\n"
      fi
      ;;
    MINGW*|MSYS*|CYGWIN*)
      printf "hint install SDL2 on MSYS2: pacman -S --needed mingw-w64-x86_64-SDL2\n"
      ;;
    *)
      printf "hint install SDL2 runtime/dev libraries for your platform.\n"
      ;;
  esac
}

check_wasm_pack() {
  if command -v wasm-pack >/dev/null 2>&1; then
    printf "ok   web frontend tool: %s\n" "$(wasm-pack --version)"
    return
  fi

  if [ "$install_wasm_pack" -eq 1 ]; then
    printf "info installing wasm-pack via cargo...\n"
    if cargo install wasm-pack; then
      if command -v wasm-pack >/dev/null 2>&1; then
        printf "ok   web frontend tool: %s\n" "$(wasm-pack --version)"
        return
      fi
    fi
    printf "miss web frontend tool: wasm-pack (installation failed)\n"
    mark_missing
    return
  fi

  printf "miss web frontend tool: wasm-pack\n"
  printf "hint install: cargo install wasm-pack\n"
  mark_missing
}

check_sdl2_libs() {
  if command -v sdl2-config >/dev/null 2>&1; then
    printf "ok   SDL2 frontend libs: %s\n" "$(sdl2-config --version)"
    return
  fi

  if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists sdl2; then
    printf "ok   SDL2 frontend libs: %s\n" "$(pkg-config --modversion sdl2)"
    return
  fi

  printf "miss SDL2 frontend libs\n"
  print_sdl2_hint
  mark_missing
}

printf "== Core dependencies ==\n"
check_required_cmd git
check_required_cmd curl
check_required_cmd unzip
check_required_cmd perl
check_required_cmd rg
check_required_cmd cargo
check_required_cmd rustc

if [ "$check_sdl2" -eq 1 ]; then
  printf "\n== SDL2 frontend dependencies ==\n"
  check_sdl2_libs
fi

if [ "$check_web" -eq 1 ]; then
  printf "\n== Web frontend dependencies ==\n"
  check_wasm_pack
fi

if [ "$ready" -eq 1 ]; then
  printf "\nEnvironment bootstrap check passed.\n"
  exit 0
fi

printf "\nEnvironment bootstrap check failed. Resolve missing dependencies and run again.\n" >&2
exit 1
