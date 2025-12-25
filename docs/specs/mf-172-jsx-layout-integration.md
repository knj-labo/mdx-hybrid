# MF-172: JSX layout/wrapper integration

Status: Draft

## Goal
Specify how Markdown→JSX output is wrapped with layout/components, including props, slots, and hoisted imports.

## Scope
- Layout wrapping contract (default export vs Astro component factory).
- Passing `frontmatter`, `headings`, `components` (MDX-style) into wrappers.
- Interaction with hoisted imports/exports and runtime imports.

## Open Questions
- Should layout be opt-in via frontmatter `layout` or API option?
- Slot semantics: support only `default`, or mirror Astro slots?
- Where to inject runtime helpers (createComponent, renderJSX) in JSX mode?
- Separate Starlight-specific heading wrappers/anchors from Astro-generic output.

## Proposed Behavior (current plan)
- Exports (baseline):
  - `export const frontmatter = {...}`
  - `export const file = "<abs path>"`
  - `export const url = "<route or undefined>"`
  - `export function getHeadings() { return [...] }`
- `const MarkflowContent = createComponent((result, props) => renderJSX(result, <>...</>), file)`
- `export const Content = MarkflowContent`
- `export default` is a `createComponent(...)` wrapper that renders Layout + children directly (layout present) or `MarkflowContent` (no layout)
- Layout wrapping:
  - When frontmatter contains `layout`, emit `import Layout from "<layout path>";`
  - Default export becomes:
    ```jsx
    export default createComponent(
      (result, props) =>
        renderJSX(
          result,
          _jsx(Layout, {
            ...props,
            frontmatter,
            children: _jsx(MarkflowContent, { ...props }),
          })
        ),
      file
    );
    ```
  - If no layout:
    ```jsx
    export default MarkflowContent;
    ```
 Runtime helpers:
  - Emit `import { Fragment, jsx as __jsx } from 'astro/jsx-runtime';` at the top of the module.
  - Derive `_Fragment` from `Fragment`: `const _Fragment = Fragment;`.
  - Wrap `_jsx` so `props` defaults to `{}`: `const _jsx = (type, props, key) => __jsx(type, props ?? {}, key);`.
  - Emit `import { createComponent, renderJSX } from 'astro/runtime/server/index.js';`.
  - Do not use `markHTMLString` in JSX mode.
  - Keep the runtime import before any hoisted imports/exports.
  - Limit runtime symbols to `_Fragment` and `_jsx` (no `jsxs` / `jsxDEV`).
- IR payload:
  - `CompileIrResult.html` holds the JSX source string (name retained for compatibility).
- Slots/children:
  - Only `default` slot is supported in this phase; nested/ named slots are out-of-scope.
- Hoisted imports:
  - Hoisted `import`/`export` from Markdown remain at the top of the module, before runtime/layout imports if possible; conflict resolution (same names) is out-of-scope.

## Next Steps
- Finalize wrapper templates for layout/no-layout and include in codegen.
- Document ordering of imports: runtime → layout → hoisted → generated consts → exports.
- Add fixtures: (a) layout present, (b) layout absent, (c) frontmatter without layout but hoisted imports, (d) children-only body.
