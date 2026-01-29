# find_tag_end: String Literal Tracking in JSX Expressions

**Date:** 2026-01-29
**Status:** Implemented
**File:** `crates/core/src/codegen.rs`

## Problem

`find_tag_end` didn't track string literals inside JSX expression contexts (`brace_depth > 0`). When a `<Code code={...} />` tag contained code with unbalanced braces (e.g., `"if (x) {\n  ..."`), `find_tag_end` miscounted braces, returned `None`, and subsequent `{` in text content were left unescaped, causing esbuild errors.

### Root cause

1. `slotChildrenToHtml` renders code blocks as `<Code code={JSON.stringify(code)} />`
2. Slot has PascalCase tags → `hasNestedComponents=true` → goes through `htmlEntitiesToJsx`
3. `find_tag_end` scans `<Code code={"if (x) {\n  ..."} />` — the `{` in the JSON string increments `brace_depth` but no matching `}` exists → returns `None`
4. Everything after is appended as-is, braces in subsequent text content are NOT escaped → JSX parse error

## Decision

Add string literal tracking inside JSX expression contexts in `find_tag_end`. When `brace_depth > 0`, track `"` and `'` as string delimiters with `\` escape handling (JSON strings use `\"`).

## Alternatives considered

1. **Escape `{`/`}` in `escapeHtml`** — Only covers the non-`ecComponent` path in `slotChildrenToHtml`. When `ecComponent` exists, `<Code code={...} />` is generated and goes through `htmlEntitiesToJsx`, so this doesn't fix the root cause.

2. **Force `set:html` path in `slotChildrenToHtml` when `ecComponent` is present** — Would bypass nested component detection but breaks Astro's slot processing.

3. **Track string literals in `find_tag_end` (chosen)** — Directly fixes the root cause. Distinguishes HTML attribute quotes (`brace_depth==0`, no escape handling needed) from JSX expression strings (`brace_depth>0`, `\` escape handling for JSON).

## Why backslash escape handling is needed

`JSON.stringify` escapes `"` as `\"`. Without tracking `\` escapes, `find_tag_end` would treat `\"` as a string terminator, causing subsequent `}` to incorrectly decrement brace depth. Example: `code={"a \" } b"}` — without escape handling, `\"` ends the string, ` } ` decrements to depth 0, and the tag end is lost.
