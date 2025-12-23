# codex-sdd

[![CI][ci-badge]][ci-link]
[![License: MIT][license-badge]][license-link]

codex-sdd is a Rust CLI that supports a spec-driven development (SDD) workflow for Codex-powered changes. It scaffolds the SDD documentation tree, runs Codex prompts to generate repo digests/reviews/tasks, and manages multi-agent worktrees, testing, selection, and finalization.

## Why this project
SDD helps teams make changes explicit, reviewable, and reproducible. codex-sdd provides the tooling to:
- Generate structured context for reviewers and implementers.
- Enforce approval gates before changes move forward.
- Track test plans, coverage, and selection rationale per agent.

## Features
- Scaffolds `docs/sdd` and `AGENTS.md` for SDD workflows.
- Builds a repo index and shards files across multiple Codex reader agents.
- Generates review and task docs from the repo digest.
- Gates work with explicit approval, then creates per-agent git worktrees.
- Runs tests and optional coverage, captures metrics, and summarizes variants.
- Finalizes by merging/cherry-picking a selected agent branch and archiving the change.
- Provides a CI-friendly `check` command to enforce required artifacts.

## Requirements
- Rust toolchain (stable) with `cargo`
- A git repository (commands use `git` under the hood)
- Codex CLI available as `codex`
- Optional: `cargo llvm-cov` or `cargo tarpaulin` for coverage
- Optional: Node.js 18+ for npm-based installation

## Install
We recommend npm for global installs and upgrades. Other options are available if npm is not a fit.

### npm (Node 18+) — recommended
```bash
npm install -g codex-sdd
```

If the binary is not found, install the platform package explicitly:
```bash
# macOS
npm install -g @codex-sdd/darwin-arm64
npm install -g @codex-sdd/darwin-x64

# Linux
npm install -g @codex-sdd/linux-x64
npm install -g @codex-sdd/linux-arm64

# Windows
npm install -g @codex-sdd/win32-x64
```

Why npm -g as the primary path?
- Codex CLI itself is officially documented with both npm and Homebrew; npm global install/upgrade is a first-class path. (OpenAI Developers)
- npm `bin` entries are linked into the global PATH on `-g` installs, which fits CLI distribution well. (npm docs)
- `npm install -g` is explicitly documented as global mode, which is easy to explain for CI/internal environments. (npm docs)

### Homebrew (macOS/Linux)
```bash
brew tap <owner>/tap
brew install codex-sdd
```

### Cargo (from source or git)
```bash
cargo install --path .
# or
cargo install --git <repo_url> --locked
```

### Manual (prebuilt binary)
Download the release asset for your OS/arch and place it on your PATH:
```bash
tar -xzf codex-sdd-<platform>.tar.gz
sudo install -m 755 codex-sdd /usr/local/bin/codex-sdd
```

## Quickstart
```bash
codex-sdd install
codex-sdd init
codex-sdd plans --name "change-name" --agents 4
codex-sdd review
codex-sdd tasks
codex-sdd approve
codex-sdd worktrees --agents 2
codex-sdd test-plan --coverage llvm-cov
codex-sdd select
codex-sdd finalize --agent agent1
```

Check the CLI help:
```bash
codex-sdd --help
```

## 5-Minute Walkthrough
This is the shortest path to see the workflow end-to-end in a repo.

1. Set up prompts and scaffolding:
```bash
codex-sdd install
codex-sdd init
```

2. Create a change workspace and repo digest:
```bash
codex-sdd plans --name "hello-sdd" --agents 1
```
This writes `docs/sdd/changes/<id>_hello-sdd/10_repo_digest.md`.

3. Generate review and tasks:
```bash
codex-sdd review
codex-sdd tasks
```

4. Approve and create a worktree:
```bash
codex-sdd approve
codex-sdd worktrees --agents 1
```

5. Optional end-to-end checks and finalize:
```bash
codex-sdd test-plan --coverage none
codex-sdd select
codex-sdd finalize --agent agent1
```

If you only want the documentation artifacts, you can stop after `codex-sdd tasks`.

## Usage flow
1. `install` and `init` to set up prompts and SDD scaffolding.
2. `plans` to index the repo and create a change workspace.
3. `review` and `tasks` to generate structured docs.
4. `approve` to unlock implementation work.
5. `worktrees` to create per-agent branches.
6. `test-plan` to run tests and optional coverage per agent.
7. `select` to compare variants.
8. `finalize` to merge the chosen agent branch and archive the change.

## Commands
- `install`: Write `CODEX_HOME/prompts/plans.md` for Codex sessions.
- `init`: Scaffold `docs/sdd` and ensure `AGENTS.md` exists.
- `plans`: Create a change workspace, index files, and run reader agents.
  - `--name` (required), `--id` (optional), `--agents` (default 4), `--include-untracked`
- `review`: Generate `20_review.md` from the repo digest.
- `tasks`: Generate `40_tasks.md` from the repo digest and review.
- `approve`: Record approval and write `90_decision.md`.
- `worktrees`: Create per-agent git worktrees after approval.
  - `--agents` (default 2)
- `test-plan`: Generate test plans, run `cargo test`, and optional coverage.
  - `--coverage` = `llvm-cov` (default), `tarpaulin`, or `none`
- `select`: Summarize variants (tests, coverage, diff size) into `80_selection.md`.
- `finalize`: Merge/cherry-pick the selected agent branch and archive the change.
  - `--agent` (required), `--strategy` = `merge` (default) or `cherry-pick`
- `check`: CI gate for required spec updates and artifacts.

## Directory layout
```
docs/sdd/
  specs/          # Current specs
  changes/        # In-flight change sessions
  archive/        # Completed changes
.codex/sdd/
  state.json      # Internal state
  runs/           # Codex outputs, metrics
  worktrees/      # Per-agent worktrees
  schemas/        # JSON schemas for outputs
```

Each change folder under `docs/sdd/changes/<id>_<name>/` includes:
- `10_repo_digest.md`
- `20_review.md`
- `40_tasks.md`
- `50_test_plan.md`
- `80_selection.md`
- `90_decision.md`

## Configuration
- `CODEX_HOME`: Base directory for Codex assets (default: `~/.codex`).
- `CODEX_SDD_PROMPT_FLAG`: Override the prompt flag (default: `--prompt-file`).
- `CODEX_SDD_EXEC_ARGS`: Extra args passed to `codex exec`.

## CI check behavior
`codex-sdd check` passes immediately when only `docs/**` files changed. If code changes are detected, it requires:
- An updated `docs/sdd/specs/*.md`
- Change artifacts: `90_decision.md`, `40_tasks.md`, and `50_test_plan.md`

## Development
```bash
cargo test
```

## Contributing
Contributions are welcome. Please open an issue to discuss major changes before submitting a PR. By contributing, you agree that your work will be licensed under the project license.

## Notes
- Consider adding `.codex/sdd/` to `.gitignore` to avoid committing runtime state (do not ignore `.codex/skills` if you use it).

## License
MIT

[ci-badge]: https://github.com/<owner>/<repo>/actions/workflows/ci.yml/badge.svg
[ci-link]: https://github.com/<owner>/<repo>/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-link]: LICENSE
