# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mdx-hybrid is a hybrid MDX compiler that combines Rust's compilation speed with JavaScript's plugin compatibility. It provides 5-10x build time improvements for projects without plugins by intelligently selecting between two engines:

- **Rust Engine**: Fast native compilation using mdxjs-rs via napi-rs bindings
- **JS Engine**: Full compatibility wrapper around @mdx-js/mdx supporting all plugins

## Development Commands

### Initial Setup
```bash
# Install dependencies
pnpm install

# Build all packages
pnpm build

# Set up git hooks
pnpm prepare
```

### Development Workflow
```bash
# Start development mode
pnpm dev           # Watch and rebuild on changes

# Lint and format code with Biome
pnpm lint          # Check and auto-fix issues
pnpm format        # Format code only

# Type checking
pnpm typecheck     # Check all packages

# Testing
pnpm test          # Run all tests
pnpm test:ui       # Run tests with Vitest UI
pnpm bench         # Run performance benchmarks

# Clean build artifacts
pnpm clean         # Clean all packages
```

### Release Process
```bash
# Create a changeset for your changes
pnpm changeset

# Update package versions based on changesets
pnpm version

# Build and publish to npm
pnpm release
```

## Architecture

### Package Structure
```
packages/
├── core/                   # Main API and engine selection
│   ├── src/
│   │   ├── index.ts       # Public API exports
│   │   ├── compiler.ts    # Compiler class implementation
│   │   ├── engine-selector.ts # Engine selection logic
│   │   ├── types.ts       # TypeScript interfaces
│   │   └── errors.ts      # Custom error classes
│   └── tests/
│
├── engines/
│   ├── js/                # JavaScript engine
│   │   ├── src/
│   │   │   ├── index.ts   # Engine interface
│   │   │   ├── engine.ts  # @mdx-js/mdx wrapper
│   │   │   └── options.ts # Option transformation
│   │   └── tests/
│   │
│   └── rust/              # Rust engine
│       ├── src/
│       │   ├── lib.rs     # Rust bindings
│       │   └── options.rs # Option mapping
│       ├── index.js       # Node.js entry
│       └── Cargo.toml
│
└── integrations/
    └── vite/              # Vite plugin
```

### Engine Selection Algorithm

The engine selection logic is implemented in `packages/core/src/engine-selector.ts`:

1. **Explicit Selection**: If `options.engine` is specified ('js' | 'rust'), use that engine
2. **Plugin Detection**: If `remarkPlugins` or `rehypePlugins` exist → JS engine (required for plugin support)
3. **Binary Availability**: If Rust binary is available → Rust engine (for performance)
4. **Fallback**: Default to JS engine (always available)

### Key Interfaces

```typescript
// CompileOptions - Main configuration interface
interface CompileOptions {
  engine?: 'js' | 'rust' | 'auto'  // Engine selection (default: 'auto')
  remarkPlugins?: any[]            // Markdown plugins (forces JS engine)
  rehypePlugins?: any[]            // HTML plugins (forces JS engine)
  development?: boolean            // Development mode
  jsx?: boolean                    // Enable JSX
  jsxRuntime?: 'automatic' | 'classic'
  jsxImportSource?: string         // JSX import source
  // ... other @mdx-js/mdx compatible options
}

// CompileResult - Output format
interface CompileResult {
  code: string                     // Compiled JavaScript
  map?: object                     // Source map (v3 spec)
  timing: number                   // Compilation time (ms)
}
```

### Error Handling

| Error Type | Description | Behavior |
|------------|-------------|----------|
| `RustUnavailableError` | Rust binary not found when `engine: 'rust'` specified | Fatal error, no fallback |
| `RustPanicWarning` | Rust engine crashed during compilation | Auto-fallback to JS engine with warning |
| `CompilerError` | MDX compilation error from either engine | Fatal error with details |

## Testing Strategy

### Test Organization
- **Unit Tests**: Each package has its own test suite in `tests/`
- **Integration Tests**: Cross-package functionality in `packages/core/tests/`
- **Golden Tests**: Output comparison between JS and Rust engines
- **Performance Tests**: Benchmarks in `benchmarks/` with small/medium/large MDX files

### Running Tests
```bash
# Run specific package tests
pnpm --filter @mdx-hybrid/core test

# Run tests with coverage
pnpm test -- --coverage

# Run specific test file
pnpm test packages/core/tests/engine-selector.test.ts
```

## Performance Targets

Based on the specification, the Rust engine should achieve:

| File Size | Lines | JS Engine | Rust Engine | Target Speedup |
|-----------|-------|-----------|-------------|----------------|
| Small     | 10    | 5 ms      | 1 ms        | 5×             |
| Medium    | 100   | 25 ms     | 4 ms        | 6.25×          |
| Large     | 1000  | 150 ms    | 20 ms       | 7.5×           |

## Build Configuration

### Rust Engine Build
The Rust engine uses napi-rs for Node.js bindings:
- Prebuild binaries for all supported platforms
- Static linking (no OpenSSL dependencies)
- GitHub Actions matrix builds for OS × Architecture combinations

### Supported Platforms
- macOS: x64, arm64
- Linux: x64 (glibc), x64 (musl)
- Windows: x64

## Common Development Tasks

### Adding a New Package
1. Create directory under `packages/` or appropriate location
2. Add package.json with workspace protocol dependencies
3. Update `pnpm-workspace.yaml` if needed
4. Add to Turbo pipeline in `turbo.json`

### Implementing a New Feature
1. Create feature branch from main
2. Implement with tests
3. Run `pnpm changeset` to document changes
4. Create PR with description

### Debugging Engine Selection
```typescript
// Enable debug logging
process.env.MDX_HYBRID_DEBUG = 'true'

// Force specific engine
compile(content, { engine: 'rust' })
```

## Integration Examples

### Astro Integration
```typescript
// packages/integrations/astro/
import { mdxHybrid } from '@mdx-hybrid/core/astro'

export default defineConfig({
  integrations: [mdxHybrid()]
})
```

### Vite Plugin
```typescript
// packages/integrations/vite/
import { mdxHybrid } from '@mdx-hybrid/core/vite'

export default {
  plugins: [mdxHybrid()]
}
```

## Important Notes

1. **Plugin Compatibility**: Rust engine does NOT support remark/rehype plugins currently
2. **Binary Size**: Rust engine adds ~3-5 MB per architecture
3. **Source Maps**: Rust engine provides basic source maps only
4. **Streaming**: Not implemented in initial version
5. **AST Bridge**: Future enhancement for selective plugin support

## Git Workflow

This project uses:
- **Conventional Commits**: Enforced by commitlint
- **Changesets**: For version management
- **Husky**: For git hooks (pre-commit, pre-push)
- **Biome**: For fast linting and formatting
- **lint-staged**: Automatically formats and lints staged files on commit:
  - TypeScript/JavaScript files: Biome check and format
  - Markdown/MDX files: Biome format
  - Rust files: cargo fmt

## Troubleshooting

### Rust Binary Not Found
```bash
# Rebuild the Rust engine
pnpm --filter @mdx-hybrid/engine-rust build

# Check binary location
ls packages/engines/rust/*.node
```

### Type Errors
```bash
# Rebuild TypeScript declarations
pnpm turbo run build --filter=@mdx-hybrid/core^...
```

### Performance Issues
```bash
# Run benchmarks to verify
pnpm bench

# Check engine selection
MDX_HYBRID_DEBUG=true pnpm test
```