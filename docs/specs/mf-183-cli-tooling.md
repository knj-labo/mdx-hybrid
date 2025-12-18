# MF-183: CLI/tooling entrypoint spec

Status: Draft

## Scope
- Define CLI (if provided) or developer tooling commands for Markflow.
- Options/flags, inputs/outputs, and expected behaviors.

## Policy (initial ideas)
- Possible commands: `markflow render-html`, `markflow render-jsx`, `markflow hoist`, `markflow lint` (TBD).
- Prefer piping/streaming I/O; avoid writing files unless specified.

## Open Questions
- Do we ship a standalone CLI or only tooling scripts?
- What config resolution (cwd, .markflowrc) is needed?
- Should CLI mirror NAPI/WASM options (runtimeImport, wrapLayout, etc.)?

## Next Steps
- Decide whether to ship CLI; if yes, define subcommands and option sets.
- Add examples and integration test plan.
