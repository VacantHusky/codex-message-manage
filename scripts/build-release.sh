#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
PACKAGE_NAME="$(awk -F'"' '/^name = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
PACKAGE_VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
ENV_TARGETS="${TARGETS:-}"

DEFAULT_TARGETS=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
  "x86_64-pc-windows-gnu"
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
)

SKIP_FRONTEND=0
CLEAN=0
TARGETS=()

usage() {
  cat <<EOF
Usage: scripts/build-release.sh [options]

Options:
  --all                 Build the default desktop targets. This is the default.
  --target <triple>     Build one target. Can be repeated.
  --skip-frontend       Reuse existing frontend/dist.
  --clean               Remove dist/ before packaging.
  -h, --help            Show this help.

Environment:
  TARGETS="triple ..."  Override targets without passing --target.
  USE_CROSS=1           Use cross instead of cargo when cross is installed.

Default targets:
  ${DEFAULT_TARGETS[*]}
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)
      TARGETS=()
      shift
      ;;
    --target)
      [[ $# -ge 2 ]] || { echo "--target requires a Rust target triple" >&2; exit 2; }
      TARGETS+=("$2")
      shift 2
      ;;
    --skip-frontend)
      SKIP_FRONTEND=1
      shift
      ;;
    --clean)
      CLEAN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ${#TARGETS[@]} -eq 0 && -n "$ENV_TARGETS" ]]; then
  read -r -a TARGETS <<< "$ENV_TARGETS"
elif [[ ${#TARGETS[@]} -eq 0 ]]; then
  TARGETS=("${DEFAULT_TARGETS[@]}")
fi

if [[ "$CLEAN" == "1" ]]; then
  rm -rf "$DIST_DIR"
fi
mkdir -p "$DIST_DIR"

if [[ "$SKIP_FRONTEND" != "1" ]]; then
  echo "==> Building frontend"
  (cd "$ROOT_DIR/frontend" && npm run build)
elif [[ ! -d "$ROOT_DIR/frontend/dist" ]]; then
  echo "frontend/dist does not exist; remove --skip-frontend or build the frontend first" >&2
  exit 2
fi

if [[ "${USE_CROSS:-0}" == "1" ]] && command -v cross >/dev/null 2>&1; then
  CARGO_BIN="cross"
else
  CARGO_BIN="cargo"
fi

FAILED=()
BUILT=()

for TARGET in "${TARGETS[@]}"; do
  echo "==> Building $TARGET"
  if command -v rustup >/dev/null 2>&1; then
    rustup target list --installed | grep -Fxq "$TARGET" || rustup target add "$TARGET"
  fi

  if ! (cd "$ROOT_DIR" && "$CARGO_BIN" build --release --target "$TARGET"); then
    echo "!! Failed to build $TARGET" >&2
    FAILED+=("$TARGET")
    continue
  fi

  BIN_NAME="$PACKAGE_NAME"
  if [[ "$TARGET" == *"windows"* ]]; then
    BIN_NAME="$PACKAGE_NAME.exe"
  fi

  BIN_PATH="$ROOT_DIR/target/$TARGET/release/$BIN_NAME"
  if [[ ! -f "$BIN_PATH" ]]; then
    echo "!! Missing binary after build: $BIN_PATH" >&2
    FAILED+=("$TARGET")
    continue
  fi

  STAGE_NAME="$PACKAGE_NAME-$PACKAGE_VERSION-$TARGET"
  STAGE_DIR="$DIST_DIR/$STAGE_NAME"
  ARCHIVE="$DIST_DIR/$STAGE_NAME.tar.gz"

  rm -rf "$STAGE_DIR" "$ARCHIVE"
  mkdir -p "$STAGE_DIR/frontend"
  cp "$BIN_PATH" "$STAGE_DIR/"
  cp -R "$ROOT_DIR/frontend/dist" "$STAGE_DIR/frontend/dist"
  [[ -f "$ROOT_DIR/codex-manager.toml" ]] && cp "$ROOT_DIR/codex-manager.toml" "$STAGE_DIR/"
  [[ -f "$ROOT_DIR/README.md" ]] && cp "$ROOT_DIR/README.md" "$STAGE_DIR/"

  tar -C "$DIST_DIR" -czf "$ARCHIVE" "$STAGE_NAME"
  rm -rf "$STAGE_DIR"
  BUILT+=("$ARCHIVE")
done

echo
if [[ ${#BUILT[@]} -gt 0 ]]; then
  echo "Built packages:"
  printf '  %s\n' "${BUILT[@]}"
fi

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo "Failed targets:" >&2
  printf '  %s\n' "${FAILED[@]}" >&2
  echo "Tip: non-native Linux, Windows, and macOS targets may need a linker toolchain; try USE_CROSS=1 or install the target linker." >&2
  exit 1
fi
