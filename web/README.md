# Markflow Web (Astro v5) Quickstart

## Setup
- `pnpm install --filter @markflow/web`

## Develop
- `pnpm --filter @markflow/web dev`
- Astro dev server (default: http://localhost:4321)

## Build
- `pnpm --filter @markflow/web run build`

## Content
- Content Collections: `src/content/` (`config.ts` defines the `docs` collection).
- Seed document: `src/content/docs/hello-world.md` (replace with real docs as they grow).

## Notes
- Build output: `web/dist/`
- Warnings are filtered in `astro.config.mjs` to keep CI logs clean.
