<!-- OPENSPEC:START -->

# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:

- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:

- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Project Agent Instructions (codex-sdd)

## 1) Language selection
- Start by asking the user which language to use (e.g., English / Japanese).
- After the user selects, respond only in that language.

## 2) Context7 usage
- If the task involves a library/framework and Context7 docs are available, resolve the
  library ID and use Context7 as the authoritative source before coding or explaining.

## 3) codex-sdd usage (detailed)

### Prerequisites
- Rust toolchain (cargo)
- A git repository (commands use `git` under the hood)
- Codex CLI available as `codex`
- Optional: `cargo llvm-cov` or `cargo tarpaulin` for coverage

### Setup
1. `codex-sdd install` to create `CODEX_HOME/prompts/plans.md`.
2. `codex-sdd init` to scaffold `docs/sdd` and `AGENTS.md`.

### End-to-end workflow
1. Start a change session:
   - `codex-sdd plans --name "<change-name>" [--agents N] [--include-untracked]`
   - Output: `docs/sdd/changes/<id>_<name>/10_repo_digest.md`
2. Generate review and tasks:
   - `codex-sdd review`
   - `codex-sdd tasks`
   - Outputs: `20_review.md`, `40_tasks.md`
3. Approve to unlock work:
   - `codex-sdd approve`
   - Output: `90_decision.md`
4. Create worktrees:
   - `codex-sdd worktrees --agents N`
   - Output: `.codex/sdd/worktrees/<change_id>/agentN`
5. Test plan and execution:
   - `codex-sdd test-plan [--coverage llvm-cov|tarpaulin|none]`
   - Output: `50_test_plan.md` + metrics in `.codex/sdd/runs/<change_id>/`
6. Compare variants:
   - `codex-sdd select`
   - Output: `80_selection.md`
7. Finalize:
   - `codex-sdd finalize --agent agent1 [--strategy merge|cherry-pick]`
   - Requires `docs/sdd/specs/<spec>.md` update when code changed.
   - Output: change archived to `docs/sdd/archive/<date>-<change_dir>`

### CI check rules
- `codex-sdd check` passes if only `docs/**` changed.
- For code changes, it requires:
  - `docs/sdd/specs/*.md` updated
  - `90_decision.md`, `40_tasks.md`, `50_test_plan.md` present under a change directory.

### Generated paths
- `docs/sdd/specs/` current specs
- `docs/sdd/changes/` active sessions
- `docs/sdd/archive/` completed changes
- `.codex/sdd/` runtime state (state.json, runs, worktrees, schemas)

### Environment variables
- `CODEX_HOME`: Base directory for Codex assets (default: `~/.codex`).
- `CODEX_SDD_PROMPT_FLAG`: Override the prompt flag (default: `--prompt-file`).
- `CODEX_SDD_EXEC_ARGS`: Extra args passed to `codex exec`.
