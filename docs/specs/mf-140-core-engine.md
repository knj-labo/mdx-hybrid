# MF-140 Core Engine Streaming Adapter

## Background
Phase 1 (Core Engine Refactoring) in [ROADMAP.md](../../ROADMAP.md) requires replacing the prototype parser with a streaming, markdown-rs-backed pipeline. The old adapter materialized every Markdown event into a `Vec<Event>` before rendering, which blocked lol_html from streaming, increased memory usage on large files, and made tight-list handling diverge from GFM. This spec documents the finished streaming iterator so WASM/N-API bindings inherit the same zero-copy behavior.

## Owner & Timeline
- Owner: Kenji (Core)
- Reviewer: Aya (Bindings)
- Target ship date: 2026-01-15 (unblocks Phase 1 sign-off and the WASM streaming follow-up)

## Requirements
- Replace the `EventBuilder` + `Vec<Event>` buffer in `crates/core/src/markdown_adapter.rs` with a cursor that walks the `mdast::Node` tree lazily and emits events directly into the `MarkdownStream` adapter.
- Preserve all existing features (frontmatter passthrough, math nodes, tables, footnotes, task lists, heading slug generation) and keep image/link semantics identical to the pre-refactor implementation.
- Track tight-list state so paragraphs are removed only when CommonMark marks a list item as tight; loose items must still emit `<p>` tags.
- Keep the public API unchanged (`get_event_iterator`, `MarkdownEventStream`, `MarkdownStream`) so downstream crates require no updates.
- Add targeted unit tests covering the new iterator invariants (tight vs. loose lists, task-list marker ordering) to prevent regressions.

## Definition of Done
1. `MarkdownParser` streams events directly into `HtmlRenderer`/`StreamingRewriter` without allocating an intermediate `Vec<Event>`.
2. `cargo test --package markflow-core` passes locally and in CI.
3. The new tests cover both tight and loose lists plus task-list marker ordering, and the module docs explain the iterator’s behavior after the refactor.
4. Backlog and spec references remain in sync by running `node scripts/check-backlog.mjs`.

## Acceptance Tests
1. `cargo test --package markflow-core markdown_adapter::tests::tight_list_strips_paragraphs`
2. `cargo test --package markflow-core markdown_adapter::tests::loose_list_retains_paragraphs`
3. `cargo test --package markflow-core markdown_adapter::tests::task_list_emits_marker_before_text`
4. `node scripts/check-backlog.mjs`

## References
- ROADMAP Phase 1 notes (`ROADMAP.md`)
- Architecture overview (`docs/architecture/overview.md`)
- Spec-driven workflow guide (`docs/development/spec-driven-codex.md`)
