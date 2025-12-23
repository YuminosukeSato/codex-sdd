# Spec-Driven Development (SDD)

このフォルダは `codex-sdd` が生成する成果物の置き場です。仕様を中心に、提案・作業・承認をファイルで管理します。

## 構成

- `specs/`: 現行仕様（デプロイ済み）
- `changes/`: 進行中の提案と作業セッション
- `archive/`: 完了済みの変更

## 承認ゲート

- `codex-sdd approve` で `90_decision.md` と `.codex/sdd/state.json` を更新します。
- `worktrees` / `test-plan` / `finalize` は承認済みの変更のみ実行できます。

## キャッシュと再実行

- `plans` はファイルハッシュとシャードハッシュを記録し、変更がないシャードは再実行をスキップします。
- キャッシュは `.codex/sdd/state.json` と `.codex/sdd/runs/` に保存されます。

## 注意

- `.codex/sdd/` は `.gitignore` に追加してください（`.codex/skills` は除外しない）。
- 生成物の順序は決定的（ソート済み）です。
