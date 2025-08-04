# Platform-Specific Binary Packages

This directory contains the platform-specific npm packages for the mdx-hybrid Rust engine binaries.

## Structure

Each subdirectory represents a platform-specific package that will be published to npm:

- `darwin-arm64/` - macOS ARM64 (Apple Silicon)
- `darwin-x64/` - macOS x64 (Intel)
- `darwin-universal/` - macOS Universal (ARM64 + x64)
- `win32-x64-msvc/` - Windows x64 (MSVC)
- `linux-x64-gnu/` - Linux x64 (glibc)
- `linux-x64-musl/` - Linux x64 (musl)

## Publishing Process

The binary packages are published automatically when a new version is released:

1. Main release workflow publishes the core packages
2. Binary publishing workflow is triggered automatically
3. Each platform binary is built and published as a separate npm package
4. The main `@jp-knj/mdx-hybrid-engine-rust` package references these as optional dependencies

## Manual Publishing

For manual publishing (e.g., fixing a specific platform):

```bash
# From packages/engines/rust directory

# Build the binary for your platform
pnpm build

# Publish binaries for current version
node scripts/publish-binaries.js

# Or specify a version
node scripts/publish-binaries.js 0.0.4

# Dry run to test
node scripts/publish-binaries.js 0.0.4 --dry-run
```

## Version Management

The version in each platform package's `package.json` should match the version of the main `@jp-knj/mdx-hybrid-engine-rust` package. This is handled automatically by the publishing workflow.

## Binary Naming Convention

Binary files follow the napi-rs naming convention:
- `mdx-hybrid-engine-rust.{platform}-{arch}.node`

Examples:
- `mdx-hybrid-engine-rust.darwin-arm64.node`
- `mdx-hybrid-engine-rust.win32-x64-msvc.node`
- `mdx-hybrid-engine-rust.linux-x64-gnu.node`