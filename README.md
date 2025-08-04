# mdx-hybrid

A hybrid MDX compiler that combines Rust's compilation speed with JavaScript's plugin compatibility, providing 5-20x build time improvements for projects without plugins.

## 🎉 Status

**v0.0.1** - Fully functional with Rust engine delivering impressive performance gains!

## Features

- 🚀 **Blazing Fast**: 5-20x faster compilation with Rust engine (benchmarked)
- 🔧 **Full Compatibility**: Falls back to JS engine for plugin support
- 🎯 **Smart Selection**: Automatically chooses the best engine
- 📦 **Drop-in Replacement**: Works with existing MDX setups
- 🔌 **Framework Integrations**: Vite, Astro, and more

## Performance

Real-world benchmark results:
- **Small files (< 1KB)**: ~20x faster with Rust
- **Medium files (2-3KB)**: ~7x faster with Rust
- **Large files (> 10KB)**: ~5x faster with Rust

## Installation

```bash
npm install @jp-knj/mdx-hybrid-core
# or
pnpm add @jp-knj/mdx-hybrid-core
# or
yarn add @jp-knj/mdx-hybrid-core
```

## Usage

### Basic Usage

```javascript
import { compile } from '@jp-knj/mdx-hybrid-core'

const result = await compile('# Hello MDX!')
console.log(result.code)
```

### With Options

```javascript
import { compile } from '@jp-knj/mdx-hybrid-core'

const result = await compile(mdxContent, {
  // Force specific engine
  engine: 'rust', // 'js' | 'rust' | 'auto' (default)
  
  // Standard MDX options
  development: true,
  jsx: true,
  jsxRuntime: 'automatic',
  jsxImportSource: 'react',
})
```

### Vite Integration

```javascript
// vite.config.js
import { mdxHybrid } from '@jp-knj/mdx-hybrid-vite'

export default {
  plugins: [mdxHybrid()]
}
```

### Astro Integration

```javascript
// astro.config.mjs
import { mdxHybrid } from '@jp-knj/mdx-hybrid-astro'

export default {
  integrations: [mdxHybrid()]
}
```

## Engine Selection

The compiler automatically selects the appropriate engine:

1. **Explicit**: Use the engine specified in options (`engine: 'rust'` or `engine: 'js'`)
2. **Plugins**: Automatically use JS engine if remark/rehype plugins are present
3. **Performance**: Use Rust engine by default for maximum speed
4. **Fallback**: Gracefully fall back to available engine if one fails

### When Each Engine is Used

- **Rust Engine** ⚡️
  - No plugins required
  - Maximum compilation speed needed
  - Production builds without customization

- **JS Engine** 🔌
  - Remark/Rehype plugins needed
  - Custom transformations required
  - Development with hot reload (still fast!)

## Compatibility

| Feature | JS Engine | Rust Engine |
|---------|-----------|-------------|
| Basic MDX | ✅ | ✅ |
| remarkPlugins | ✅ | ❌ |
| rehypePlugins | ✅ | ❌ |
| JSX Runtime | ✅ | ✅ |
| Development Mode | ✅ | ✅ |
| Source Maps | ✅ | ⚠️ Basic |

## Binary Distribution

The Rust engine uses platform-specific native binaries for optimal performance. These binaries are automatically downloaded during installation based on your platform.

### Supported Platforms

| Platform | Architecture | Package |
|----------|-------------|---------|
| macOS | x64 (Intel) | `@jp-knj/mdx-hybrid-engine-rust-darwin-x64` |
| macOS | ARM64 (Apple Silicon) | `@jp-knj/mdx-hybrid-engine-rust-darwin-arm64` |
| Windows | x64 | `@jp-knj/mdx-hybrid-engine-rust-win32-x64-msvc` |
| Linux | x64 (glibc) | `@jp-knj/mdx-hybrid-engine-rust-linux-x64-gnu` |
| Linux | x64 (musl) | `@jp-knj/mdx-hybrid-engine-rust-linux-x64-musl` |

The correct binary is automatically selected during installation via npm's `optionalDependencies`.

## Development

```bash
# Install dependencies
pnpm install

# Build all packages
pnpm build

# Run tests
pnpm test

# Run benchmarks
pnpm bench

# Start development mode
pnpm dev
```

### Publishing Binaries

The project uses GitHub Actions to automatically build and publish platform-specific binaries:

```bash
# Build Rust engine for current platform
pnpm --filter @jp-knj/mdx-hybrid-engine-rust build

# Manually publish binaries (requires npm access)
pnpm --filter @jp-knj/mdx-hybrid-engine-rust publish:binaries

# Dry run to test publishing
pnpm --filter @jp-knj/mdx-hybrid-engine-rust publish:binaries -- --dry-run
```

Binary publishing is automated via GitHub Actions when releasing a new version.

## License

MIT
