# @jp-knj/mdx-hybrid-engine-js

## 0.0.3

### Patch Changes

- Add minimal required fixes for v0.0.3

  - ✅ JSX pragma imports are always included in output (both engines)
  - ✅ Added `data` field to CompileResult interface for VFile data storage
  - ✅ Export format unified as `export default` (both engines already compliant)

  The `data` field contains VFile data from the JS engine (frontmatter, exports, etc.) and is undefined for the Rust engine (which doesn't currently provide this data).

## 0.0.2

### Patch Changes

- Fix Rust engine integration and improve error handling

  - Improved error handling for JS engine ESM/CJS compatibility issues
  - Added graceful fallback when JS engine fails but Rust engine is available
  - Fixed engine router to better handle sync operations with preloading
  - Updated benchmarks to properly preload engines before testing
  - Confirmed Rust engine delivers 5-20x performance improvements
  - Enhanced auto-selection logic to prefer Rust engine when available

## 0.0.1

### Patch Changes

- Initial experimental release for Astro PoC

  - Core package with hybrid MDX compilation
  - Rust engine for 5-10x faster builds
  - JS engine for full plugin compatibility
  - Vite and Astro integrations
  - ESM-only architecture with preloadEngines() for sync support
