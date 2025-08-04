import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm', 'cjs'],
  dts: true,
  clean: true,
  outExtension({ format }) {
    return {
      js: format === 'esm' ? '.mjs' : '.js',
    }
  },
  external: [
    '@jp-knj/mdx-hybrid-engine-rust-win32-x64-msvc',
    '@jp-knj/mdx-hybrid-engine-rust-darwin-x64',
    '@jp-knj/mdx-hybrid-engine-rust-darwin-arm64',
    '@jp-knj/mdx-hybrid-engine-rust-darwin-universal',
    '@jp-knj/mdx-hybrid-engine-rust-linux-x64-gnu',
    '@jp-knj/mdx-hybrid-engine-rust-linux-x64-musl',
    '@jp-knj/mdx-hybrid-engine-rust-android-arm64',
    '@jp-knj/mdx-hybrid-engine-rust-android-arm-eabi',
    '@jp-knj/mdx-hybrid-engine-rust-win32-ia32-msvc',
    '@jp-knj/mdx-hybrid-engine-rust-win32-arm64-msvc',
    '@jp-knj/mdx-hybrid-engine-rust-freebsd-x64',
    '@jp-knj/mdx-hybrid-engine-rust-linux-arm64-gnu',
    '@jp-knj/mdx-hybrid-engine-rust-linux-arm64-musl',
    '@jp-knj/mdx-hybrid-engine-rust-linux-arm-gnueabihf',
    '@jp-knj/mdx-hybrid-engine-rust-linux-arm-musleabihf',
    '@jp-knj/mdx-hybrid-engine-rust-linux-riscv64-gnu',
    '@jp-knj/mdx-hybrid-engine-rust-linux-riscv64-musl',
    '@jp-knj/mdx-hybrid-engine-rust-linux-s390x-gnu',
  ],
  noExternal: [/^(?!@jp-knj\/mdx-hybrid-engine-rust-)/],
  treeshake: false,
  splitting: false,
  sourcemap: false,
})