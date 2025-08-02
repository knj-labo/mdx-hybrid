# MDX-Hybrid Project Architecture

## Overview
MDX-Hybrid is a hybrid MDX compiler that intelligently switches between Rust and JavaScript engines based on compilation needs.

## Core Components

### 1. Engine Selector (`packages/core/src/engine-selector.ts`)
- **EngineSelector class**: Main logic for choosing between engines
- Key methods:
  - `selectEngine()`: Determines which engine to use based on options
  - `compileWithFallback()`: Provides automatic fallback from Rust to JS on failure
  - Engine selection priority:
    1. Explicit engine option (`engine: 'js' | 'rust'`)
    2. Plugin detection (forces JS if plugins present)
    3. Rust availability check
    4. Default to JS engine

### 2. Compiler (`packages/core/src/compiler.ts`)
- **Compiler class**: High-level API wrapping engine selector
- Provides both async `compile()` and sync `compileSync()` methods
- Includes `getEngineInfo()` for debugging which engine was used

### 3. Engines

#### JS Engine (`packages/engines/js/`)
- Wrapper around `@mdx-js/mdx`
- Full plugin support (remark/rehype)
- Always available as fallback

#### Rust Engine (`packages/engines/rust/`)
- Native bindings via napi-rs
- 5-10x faster compilation
- No plugin support currently
- Platform-specific binaries

### 4. Integrations
- **Vite Plugin** (`packages/core/src/vite.ts`): 
  - Pre-enforced plugin with HMR support
  - Configurable include/exclude patterns
  - Automatic development mode detection

## Key Types
- `CompileOptions`: Main config interface supporting all @mdx-js/mdx options plus `engine` selection
- `CompileResult`: Output with code, source map, and timing info
- `Engine`: Interface implemented by both JS and Rust engines

## Error Handling
- `RustUnavailableError`: When Rust explicitly requested but not available
- `RustPanicWarning`: Rust compilation failure with auto-fallback to JS
- `CompilerError`: General compilation errors from either engine