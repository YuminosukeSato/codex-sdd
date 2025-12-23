# codex-sdd ワークフロー

## 1. インストール

```
codex-sdd install
```

`CODEX_HOME/prompts/plans.md` を作成します。新しい Codex セッションで `/prompts:plans` が有効になります。

## 2. 初期化

```
codex-sdd init
```

`AGENTS.md` と `docs/sdd/` を整備します。`.codex/sdd/` を `.gitignore` に追加してください。

## 3. 変更セッション開始

```
codex-sdd plans --name "change-name"
```

- `docs/sdd/changes/<change_id>_<name>/` が作成されます。
- インデックスと `repo_digest.md` が生成されます。

## 4. レビュー・タスク

```
codex-sdd review
codex-sdd tasks
```

レビュー結果とタスク一覧を作成します。

## 5. 承認

```
codex-sdd approve
```

承認ゲートを解除し、作業フェーズに進めます。

## 6. 作業用 worktree

```
codex-sdd worktrees --agents 2
```

各 agent 用の worktree を作成します。

## 7. テスト計画と実行

```
codex-sdd test-plan
```

テスト計画を作成し、`cargo test` を実行します。必要に応じて `--coverage` を指定してください。

## 8. 選定

```
codex-sdd select
```

テスト・差分・カバレッジを集計し、候補の比較を出力します。

## 9. 反映

```
codex-sdd finalize --agent agent1
```

選択した agent のブランチをマージ（既定: `--no-ff`）し、変更をアーカイブします。

## 10. CI チェック

```
codex-sdd check
```

- `src/**` などのコード変更がある場合、承認・タスク・テスト計画と specs 更新が必要です。
- `docs/**` のみの変更は pass します。

