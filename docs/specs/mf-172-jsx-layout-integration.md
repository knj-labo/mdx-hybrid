# MF-172: JSX layout/wrapper integration

Status: Draft

## Goal
Specify how Markdown→JSX output is wrapped with layout/components, including props, slots, and hoisted imports.

## Scope
- Layout wrapping contract (default export vs factory).
- Passing `frontmatter`, `headings`, `components` (MDX-style) into wrappers.
- Interaction with hoisted imports/exports and runtime imports.

## Open Questions
- Should layout be opt-in via frontmatter `layout` or API option?
- Slot semantics: support only `default`, or mirror Astro slots?
- Where to inject runtime helpers (createComponent, renderToString) in JSX mode?

## Proposed Behavior (current plan)
- Exports (baseline):
  - `export const frontmatter = {...}`
  - `export const file = "<abs path>"`
  - `export const url = "<route or undefined>"`
  - `export function getHeadings() { return [...] }`
  - `export const Content = /* unwrapped body component */`
  - `export default Content` (when no layout) OR wrapped layout component (when layout present)
- Layout wrapping:
  - When frontmatter contains `layout`, emit `import Layout from "<layout path>";`
  - Default export becomes:
    ```jsx
    export default function Page(props) {
      return (
        <Layout {...props} frontmatter={frontmatter}>
          <Content {...props} />
        </Layout>
      );
    }
    ```
  - If no layout: `export default Content;`
- Runtime helpers:
  - Keep existing runtime imports (`createComponent`, etc.) out-of-scope for this spec; defer to MF-150/JSX runtime spec.
- Slots/children:
  - Only `default` slot is supported in this phase; nested/ named slots are out-of-scope.
- Hoisted imports:
  - Hoisted `import`/`export` from Markdown remain at the top of the module, before runtime/layout imports if possible; conflict resolution (same names) is out-of-scope.

## Next Steps
- Finalize wrapper templates for layout/no-layout and include in codegen.
- Document ordering of imports: runtime → layout → hoisted → generated consts → exports.
- Add fixtures: (a) layout present, (b) layout absent, (c) frontmatter without layout but hoisted imports, (d) children-only body.
