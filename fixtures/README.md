# Fixtures Directory

```
fixtures/
├── core/
│   ├── markdown/        # CommonMark + legacy markdown samples for core + bindings tests
│   └── mdx/             # MDX-specific samples (embedded JSX, expressions, imports)
└── integration/
    └── astro-harness/   # Minimal Astro/Vite harness fed by markflow
```

Use `fixtures/core/*` for unit/regression suites (Rust, AVA, wasm-bindgen). Larger, framework-specific assets should live under `fixtures/integration/` as they are introduced.
