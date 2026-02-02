//! Criterion benchmarks for markflow-astro performance analysis.
//!
//! Run with: `cargo bench -p markflow-astro`
//! Or with specific benchmark: `cargo bench -p markflow-astro -- to_blocks`

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use markflow_astro::{
    renderer::MdastOptions,
    to_blocks,
    transform::{jsx_normalize, smartypants},
};
use markflow_core::slugify;

/// Sample markdown content for benchmarking - simple case
const SIMPLE_MARKDOWN: &str = r#"# Hello World

This is a simple paragraph with **bold** and *italic* text.

- Item 1
- Item 2
- Item 3
"#;

/// Sample markdown content for benchmarking - medium complexity
const MEDIUM_MARKDOWN: &str = r#"# Getting Started

Welcome to the documentation. Here's what you'll learn:

## Installation

Install the package using your preferred package manager:

```bash
npm install my-package
```

## Usage

Import and use the component:

```javascript
import { Component } from 'my-package';

function App() {
  return <Component />;
}
```

## Features

- **Fast**: Optimized for performance
- **Simple**: Easy to use API
- **Flexible**: Customizable options

> Note: This is a blockquote with some important information.

For more details, see the [API documentation](https://example.com/docs).
"#;

/// Sample markdown content for benchmarking - complex case
const COMPLEX_MARKDOWN: &str = r#"---
title: Advanced Guide
description: A comprehensive guide to advanced features
---

# Advanced Features

This guide covers advanced usage patterns and configurations.

## Table of Contents

1. [Configuration](#configuration)
2. [Plugins](#plugins)
3. [Customization](#customization)

## Configuration

### Basic Setup

First, create a configuration file:

```typescript
import type { Config } from 'my-package';

export default {
  plugins: [],
  theme: {
    colors: {
      primary: '#3498db',
      secondary: '#2ecc71',
    },
  },
} satisfies Config;
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DEBUG` | Enable debug mode | `false` |
| `PORT` | Server port | `3000` |
| `API_KEY` | Your API key | Required |

## Plugins

Plugins extend functionality. Here's an example:

```javascript
export function myPlugin() {
  return {
    name: 'my-plugin',
    setup(build) {
      build.onResolve({ filter: /\.custom$/ }, args => {
        return { path: args.path, namespace: 'custom' };
      });
    },
  };
}
```

### Available Plugins

- **analytics** - Track usage metrics
- **cache** - Enable response caching
- **compress** - Compress output files

## Customization

You can customize components using the `theme` option:

```css
.custom-class {
  --primary-color: var(--theme-primary);
  --secondary-color: var(--theme-secondary);
}
```

> **Warning**: Breaking changes may occur between major versions.

### Task Lists

- [x] Read the documentation
- [x] Install dependencies
- [ ] Configure the project
- [ ] Deploy to production

---

For questions, visit our [Discord](https://discord.example.com) or open an issue on [GitHub](https://github.com/example/repo).
"#;

/// Sample text for smartypants benchmarking
const SMARTYPANTS_TEXT: &str = r#"<p>He said, "Hello, world!" and then added, 'It's a beautiful day -- really beautiful...'</p>
<p>The price is $99.99 -- a great deal!</p>
<p>"Double quotes" and 'single quotes' --- with em-dashes...</p>
<code>"Don't transform this"</code>
<p>More "text" with 'quotes' and -- dashes...</p>"#;

/// Sample JSX content for normalization benchmarking
const JSX_CONTENT: &str = r#"Some text
<MyComponent>
  <div>
    <NestedComponent prop="value" />
  </div>
</MyComponent>

Another paragraph
<Box>
content here
</Box>

```jsx
<CodeComponent />
```

Final text
"#;

fn bench_to_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("to_blocks");
    let options = MdastOptions {
        enable_directives: true,
        ..Default::default()
    };

    group.bench_with_input(
        BenchmarkId::new("simple", "256b"),
        SIMPLE_MARKDOWN,
        |b, input| b.iter(|| to_blocks(black_box(input), black_box(&options))),
    );

    group.bench_with_input(
        BenchmarkId::new("medium", "800b"),
        MEDIUM_MARKDOWN,
        |b, input| b.iter(|| to_blocks(black_box(input), black_box(&options))),
    );

    group.bench_with_input(
        BenchmarkId::new("complex", "2kb"),
        COMPLEX_MARKDOWN,
        |b, input| b.iter(|| to_blocks(black_box(input), black_box(&options))),
    );

    group.finish();
}

fn bench_smartypants(c: &mut Criterion) {
    let mut group = c.benchmark_group("smartypants");

    group.bench_function("mixed_content", |b| {
        b.iter(|| smartypants::apply_smartypants(black_box(SMARTYPANTS_TEXT)))
    });

    // Benchmark early-exit path (no transformable characters)
    let plain_text = "<p>This is plain text without any quotes or dashes</p>";
    group.bench_function("no_transform", |b| {
        b.iter(|| smartypants::apply_smartypants(black_box(plain_text)))
    });

    group.finish();
}

fn bench_slugify(c: &mut Criterion) {
    use std::collections::HashMap;

    let mut group = c.benchmark_group("slugify");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let mut counts = HashMap::new();
            slugify(black_box("Hello World"), &mut counts)
        })
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let mut counts = HashMap::new();
            slugify(
                black_box("Getting Started with @astrojs/mdx (v2.0)"),
                &mut counts,
            )
        })
    });

    group.bench_function("unicode", |b| {
        b.iter(|| {
            let mut counts = HashMap::new();
            slugify(black_box("日本語のタイトル"), &mut counts)
        })
    });

    group.bench_function("emoji", |b| {
        b.iter(|| {
            let mut counts = HashMap::new();
            slugify(black_box("🚀 Rocket Launch"), &mut counts)
        })
    });

    group.finish();
}

fn bench_jsx_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsx_normalize");

    group.bench_function("mdx_indentation", |b| {
        b.iter(|| jsx_normalize::normalize_mdx_jsx_indentation(black_box(JSX_CONTENT)))
    });

    let wrapper_content = r#"    <p>
      <Spoiler>Hidden content here</Spoiler>
    </p>
    <p>
      <Details>More details</Details>
    </p>
"#;
    group.bench_function("collapse_wrappers", |b| {
        b.iter(|| jsx_normalize::collapse_multiline_wrapper_tags(black_box(wrapper_content)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_to_blocks,
    bench_smartypants,
    bench_slugify,
    bench_jsx_normalize
);
criterion_main!(benches);
