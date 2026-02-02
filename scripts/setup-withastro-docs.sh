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

# Apply markflow overlay (astro.config.ts and package.json with markflow integration)
OVERLAY_DIR="$REPO_ROOT/fixtures/integration/withastro-docs/overlay"
if [ -d "$OVERLAY_DIR" ]; then
  echo "Applying markflow overlay from $OVERLAY_DIR"
  cp "$OVERLAY_DIR/astro.config.ts" "$TARGET_DIR/"
  cp "$OVERLAY_DIR/package.json" "$TARGET_DIR/"
  # Copy ec.config.mjs if present (needed when expressiveCode is enabled)
  [ -f "$OVERLAY_DIR/ec.config.mjs" ] && cp "$OVERLAY_DIR/ec.config.mjs" "$TARGET_DIR/"
  # Copy src/ overlay (e.g. custom Head.astro for CSS @layer ordering)
  if [ -d "$OVERLAY_DIR/src" ]; then
    cp -R "$OVERLAY_DIR/src/." "$TARGET_DIR/src/"
  fi
  # Copy config/ overlay (custom integrations, plugins)
  if [ -d "$OVERLAY_DIR/config" ]; then
    cp -R "$OVERLAY_DIR/config/." "$TARGET_DIR/config/"
  fi
fi

echo "Done. Repository is at $TARGET_DIR (ref: $REF)"
