# @jp-knj/mdx-hybrid-engine-rust

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
