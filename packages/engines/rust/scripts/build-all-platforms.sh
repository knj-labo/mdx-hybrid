#!/bin/bash

# Script to build binaries for all supported platforms
# Note: Cross-compilation requires appropriate toolchains installed

set -e

echo "Building mdx-hybrid Rust engine for all platforms..."

TARGETS=(
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
  "x86_64-pc-windows-msvc"
  "x86_64-unknown-linux-gnu"
  "x86_64-unknown-linux-musl"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running on macOS for universal binary
IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
  IS_MACOS=true
fi

# Build each target
for target in "${TARGETS[@]}"; do
  echo -e "${YELLOW}Building for $target...${NC}"
  
  # Check if target is installed
  if ! rustup target list --installed | grep -q "$target"; then
    echo -e "${YELLOW}Installing target $target...${NC}"
    rustup target add "$target"
  fi
  
  # Build the binary
  if cargo build --release --target "$target"; then
    echo -e "${GREEN}✓ Built $target successfully${NC}"
    
    # Copy the built binary to the expected location
    case "$target" in
      "x86_64-apple-darwin")
        cp "target/$target/release/libmdx_hybrid_engine_rust.dylib" "mdx-hybrid-engine-rust.darwin-x64.node"
        ;;
      "aarch64-apple-darwin")
        cp "target/$target/release/libmdx_hybrid_engine_rust.dylib" "mdx-hybrid-engine-rust.darwin-arm64.node"
        ;;
      "x86_64-pc-windows-msvc")
        cp "target/$target/release/mdx_hybrid_engine_rust.dll" "mdx-hybrid-engine-rust.win32-x64-msvc.node"
        ;;
      "x86_64-unknown-linux-gnu")
        cp "target/$target/release/libmdx_hybrid_engine_rust.so" "mdx-hybrid-engine-rust.linux-x64-gnu.node"
        ;;
      "x86_64-unknown-linux-musl")
        cp "target/$target/release/libmdx_hybrid_engine_rust.so" "mdx-hybrid-engine-rust.linux-x64-musl.node"
        ;;
    esac
  else
    echo -e "${RED}✗ Failed to build $target${NC}"
    echo "Note: Cross-compilation may require additional setup"
  fi
done

# Create universal macOS binary if on macOS and both architectures built
if [ "$IS_MACOS" = true ] && [ -f "mdx-hybrid-engine-rust.darwin-x64.node" ] && [ -f "mdx-hybrid-engine-rust.darwin-arm64.node" ]; then
  echo -e "${YELLOW}Creating universal macOS binary...${NC}"
  lipo -create -output mdx-hybrid-engine-rust.darwin-universal.node \
    mdx-hybrid-engine-rust.darwin-x64.node \
    mdx-hybrid-engine-rust.darwin-arm64.node
  echo -e "${GREEN}✓ Created universal macOS binary${NC}"
fi

echo -e "${GREEN}Build complete!${NC}"
echo "Available binaries:"
ls -la *.node 2>/dev/null || echo "No binaries found in current directory"