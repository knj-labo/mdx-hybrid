#!/usr/bin/env bash
# Setup script for withastro-docs integration test fixture.
# See fixtures/integration/withastro-docs/README.md for details.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="$REPO_ROOT/fixtures/integration/withastro-docs/repo"

# Environment variables with defaults
REMOTE="${WITHASTRO_DOCS_REMOTE:-https://github.com/withastro/docs.git}"
REF="${WITHASTRO_DOCS_REF:-main}"
DEPTH="${WITHASTRO_DOCS_DEPTH:-1}"

# Parse arguments
RESET=false
for arg in "$@"; do
  case $arg in
    --reset)
      RESET=true
      shift
      ;;
    *)
      echo "Unknown argument: $arg"
      echo "Usage: $0 [--reset]"
      exit 1
      ;;
  esac
done

# Handle --reset flag
if [ "$RESET" = true ] && [ -d "$TARGET_DIR" ]; then
  echo "Removing existing checkout at $TARGET_DIR"
  rm -rf "$TARGET_DIR"
fi

# Clone or update
if [ -d "$TARGET_DIR/.git" ]; then
  echo "Repository already exists at $TARGET_DIR"
  echo "Fetching and checking out ref: $REF"
  cd "$TARGET_DIR"
  git fetch origin "$REF"
  git checkout FETCH_HEAD
else
  echo "Cloning $REMOTE into $TARGET_DIR"

  CLONE_ARGS=("--branch" "$REF")
  if [ -n "$DEPTH" ]; then
    CLONE_ARGS+=("--depth" "$DEPTH")
  fi

  git clone "${CLONE_ARGS[@]}" "$REMOTE" "$TARGET_DIR"
fi

echo "Done. Repository is at $TARGET_DIR (ref: $REF)"
