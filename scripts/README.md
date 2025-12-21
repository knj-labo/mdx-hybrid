# scripts/ usage quick reference

- `check-backlog.mjs` — runs backlog checks (pending items). Used before PRs when backlog changes。
- `compare-astro-harness.mjs` — compare Markflow vs baseline on Astro harness; accepts `--runs` etc.
- `run-astro-harness.mjs` — run the integration harness (`markflow|baseline`) for sanity/perf checks.
- `smoke-napi.mjs` — smoke-test N-API build against a fixture markdown (e.g., `fixtures/core/markdown/hello.md`).

現状の状態:
- 全スクリプトは残存。未使用/廃止予定は未マーク。今後不要と判断したらここに「deprecated」などを追記の上で削除を検討。 
