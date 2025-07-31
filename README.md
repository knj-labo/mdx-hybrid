# mdx-hybrid

A hybrid MDX compiler that combines Rust's compilation speed with JavaScript's plugin compatibility, providing 5-10x build time improvements for projects without plugins.

## Features

- 🚀 **Blazing Fast**: 5-10x faster compilation with Rust engine
- 🔧 **Full Compatibility**: Falls back to JS engine for plugin support
- 🎯 **Smart Selection**: Automatically chooses the best engine
- 📦 **Drop-in Replacement**: Works with existing MDX setups
- 🔌 **Framework Integrations**: Vite, Astro, and more

## Installation

```bash
npm install @mdx-hybrid/core
# or
pnpm add @mdx-hybrid/core
# or
yarn add @mdx-hybrid/core
```

## Usage

### Basic Usage

```javascript
import { compile } from '@mdx-hybrid/core'

const result = await compile('# Hello MDX!')
console.log(result.code)
```

### With Options

```javascript
import { compile } from '@mdx-hybrid/core'

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
import { mdxHybrid } from '@mdx-hybrid/vite'

export default {
  plugins: [mdxHybrid()]
}
```

### Astro Integration

```javascript
// astro.config.mjs
import { mdxHybrid } from '@mdx-hybrid/core/astro'

export default {
  integrations: [mdxHybrid()]
}
```

## Performance

| File Size | JS Engine | Rust Engine | Speedup |
|-----------|-----------|-------------|---------|
| Small (10 lines) | 5ms | 1ms | 5× |
| Medium (100 lines) | 25ms | 4ms | 6.25× |
| Large (1000 lines) | 150ms | 20ms | 7.5× |

## Engine Selection

The compiler automatically selects the appropriate engine:

1. **Explicit**: Use the engine specified in options
2. **Plugins**: Use JS engine if remark/rehype plugins are present
3. **Performance**: Use Rust engine if available
4. **Fallback**: Use JS engine as fallback

## Compatibility

| Feature | JS Engine | Rust Engine |
|---------|-----------|-------------|
| Basic MDX | ✅ | ✅ |
| remarkPlugins | ✅ | ❌ |
| rehypePlugins | ✅ | ❌ |
| JSX Runtime | ✅ | ✅ |
| Development Mode | ✅ | ✅ |
| Source Maps | ✅ | ⚠️ Basic |

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

## License

MIT
