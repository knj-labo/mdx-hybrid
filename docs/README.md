# Markflow Documentation

Welcome to the Markflow documentation. This file is the single entry point to every doc set in the repo.

## Index (where to look)

| Area | Path | Purpose |
| --- | --- | --- |
| Architecture | `docs/architecture/` | System and runtime design notes; how components fit together. |
| Decisions | `docs/decisions/` | Chronological decision log (唯一の真実). 新規の方針・例外は必ずここに記録。 |
| Specs | `docs/specs/` | ユースケースや機能仕様の詳細。実装前の参照元。 |
| CI | `docs/ci/ci-steps.md` | CI workflow stepsと保守方針のインベントリ。 |
| Repo Overview | `README.md` | What Markflow is and how to build/test at a glance. |
| Contributor Quickstart | `AGENTS.md` | コマンド・コーディング規約のクイックリファレンス。 |

## Policy

- ROADMAP.md / Backlog.md は廃止済み。最新の意思決定と経緯は `docs/decisions/` に集約する。
- 仕様を追加・変更する場合は `docs/specs/` に追記し、対応する決定を `docs/decisions/` へログする。
- 新しいドキュメントや構成変更は、必ず決定ログにリンク（決定番号）を残すこと。
