---
"@jp-knj/mdx-hybrid-core": patch
"@jp-knj/mdx-hybrid-engine-js": patch
"@jp-knj/mdx-hybrid-engine-rust": patch
---

Fix Rust engine package structure to use dist/ folder

- Restructured Rust engine to use TypeScript source and compile to dist/
- Added proper ESM and CJS builds with tsup
- Aligned all packages to use consistent dist/ folder structure
- Fixed binary loading to work from dist/ directory
- Updated all packages to version 0.0.5
- Improved npm packaging with proper .npmignore configuration

This ensures all packages have proper JavaScript wrapper code in dist/ alongside the Rust binary artifacts.