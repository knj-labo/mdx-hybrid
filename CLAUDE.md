# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Markflow is a high-performance streaming Markdown/MDX parser and compiler for Astro, built in Rust with Node.js bindings. It replaces Astro's default markdown processing with faster, more feature-rich parsing that supports directives, components, and Shiki code highlighting.

## Common Commands

### Build
```bash
cargo build                                    # Build Rust workspace
pnpm --dir crates/napi run build:napi         # Build N-API bindings (required before using packages)
pnpm --dir packages/markflow run build        # Build markflow package
pnpm --dir packages/astro-markflow run build  # Build Astro integration
```

### Test
```bash
# Rust
cargo test --workspace                         # All Rust tests
cargo test -p markflow-core                    # Core crate only
cargo test -p markflow-core test_name          # Single test by name

# JavaScript/TypeScript
pnpm --dir crates/napi test                   # N-API AVA tests
pnpm --dir crates/napi test -- --match "pat"  # AVA with pattern
pnpm --dir packages/markflow test             # markflow Bun tests
pnpm --dir packages/astro-markflow test       # astro-markflow Bun tests

# Smoke test
pnpm --dir crates/napi smoke:napi             # Quick N-API validation
```

### Lint & Format
```bash
cargo fmt --all                               # Format Rust
cargo clippy --workspace --all-targets        # Lint Rust
pnpm --dir packages/markflow run typecheck    # TypeScript typecheck
pnpm --dir packages/astro-markflow run typecheck
```

### Integration Testing (Harness)
```bash
# Astro harness comparison
pnpm --dir fixtures/integration/astro-harness install  # Install deps first
node scripts/run-astro-harness.mjs markflow            # Run Markflow build
node scripts/run-astro-harness.mjs baseline            # Run baseline build
node scripts/compare-astro-harness.mjs --mode=semantic # Compare HTML structure

# withastro/docs large-scale testing
pnpm compare:withastro-docs -- --mode=semantic         # Semantic diff
pnpm visual:withastro-docs -- --build                  # Visual regression (Playwright)
```

## Architecture

### Crates (Rust)
- **`crates/core`**: Core parser/renderer using markdown-rs. Contains parser adapters, MDAST block rendering, registry for component/directive definitions, transform pipeline (code fences, directives, smartypants).
- **`crates/napi`**: Node.js N-API bindings exposing `compileIr` and batch compilation. This is what TypeScript packages consume.
- **`crates/wasm`**: WebAssembly bindings (experimental).

### Packages (TypeScript)
- **`packages/markflow`**: Main JS/TS package with Node.js (`src/node.ts`) and browser (`src/browser.ts`) entry points. Contains registry presets (Starlight, Astro, ExpressiveCode).
- **`packages/astro-markflow`**: Astro integration with Vite plugin (`src/vite-plugin.ts`). Orchestrates transform pipeline for MDX compilation.
- **`packages/astro-loader`**: Astro Content Collections loader for markdown files.

### Data Flow
1. Markdown/MDX → `crates/core` parser (Rust) → IR blocks
2. IR → `crates/napi` → Node.js
3. `astro-markflow` Vite plugin transforms IR → JSX via `pipeline/` and `transforms/`
4. Registry presets inject framework-specific components (Starlight tabs, aside, etc.)

### Key Patterns
- **Registry-based architecture**: Component/directive definitions decoupled from core parser
- **Preset system**: Pre-configured registries for Starlight, Astro, ExpressiveCode
- **Snapshot testing**: Rust uses `insta` crate; snapshots in `crates/core/tests/snapshots/`
- **pnpm workspaces**: Always use `pnpm --dir <workspace>` to target specific packages

## Coding Conventions

- **JS/TS**: 2-space indent, single quotes, semicolons
- **Rust**: `rustfmt` defaults, `snake_case` functions, `CamelCase` types
- **Commits**: Conventional format with optional scope: `feat:`, `fix:`, `refactor(core):`, `docs(scope):`
