# MDX Hybrid - Node.js Example

This example demonstrates how to use mdx-hybrid in a Node.js environment.

## Usage

```bash
# Install dependencies
pnpm install

# Compile a single MDX file
pnpm compile

# Watch and compile MDX files
pnpm compile:watch

# Run performance benchmark
pnpm compile:benchmark
```

## Examples

### Basic Compilation
See `compile.js` for basic MDX compilation.

### Watch Mode
See `compile-watch.js` for watching and recompiling MDX files.

### Performance Testing
See `benchmark.js` for comparing JS vs Rust engine performance.