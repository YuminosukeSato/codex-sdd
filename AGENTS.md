# AGENTS.md — codex-sdd

## Purpose

This repository builds **codex-sdd**, a Spec-Driven Development (SDD) workflow tool for Codex CLI:

- Spec-first artifacts live under `docs/sdd/`.
- All “agent” work is orchestrated via **`codex exec`** and persisted as files.
- Code-changing commands are gated behind an explicit approval state recorded in `.codex/sdd/state.json`.

## Non-negotiables

1. **Specs are the source of truth.**  
   If behavior is unclear, update/add a spec or change proposal under `docs/sdd/` _before_ modifying code.
2. **No hidden side effects.**  
   Only `codex-sdd install` may write to the user’s Codex home (prompt installation). All other commands must be repo-local.
3. **Approval gate is real.**  
   Any operation that can change source code (including `worktrees`, `test-plan` if it writes tests, and `finalize`) must require `approved=true` in `.codex/sdd/state.json` (or an equivalent explicit approval record).
4. **Everything is auditable.**  
   Every `codex exec` run must write:
   - raw streaming output (JSONL when `--json` is used),
   - a final structured artifact (when using `--output-schema`),
   - and a stable manifest (inputs, flags, timestamps, hashes).

## What Codex should do in this repo

When asked to help:

1. **Locate the current accepted spec** in `docs/sdd/specs/`.
2. If proposing changes, create/update a proposal workspace in `docs/sdd/changes/<id>_<name>/`.
3. Produce outputs as files (prefer `docs/sdd/changes/...`) rather than only chat text.
4. Do not implement code changes unless:
   - the change proposal is reviewed, and
   - approval is recorded in `.codex/sdd/state.json`.

## Repository conventions (expected)

- `docs/sdd/specs/` — accepted specs (authoritative)
- `docs/sdd/changes/<id>_<name>/` — per-change workspace (plans, reviews, tasks, test designs)
- `.codex/sdd/state.json` — per-repo state (current change, approval flag, cache hashes)
- `.codex/sdd/runs/<change_id>/` — captured `codex exec` outputs per role/agent

If any of these paths do not exist yet, create them via the CLI scaffolding (preferred) or minimally via filesystem changes.

## How we “run agents”

`codex-sdd` must use **`codex exec`** (non-interactive) as the only agent runtime.

### Required runtime behaviors

- Use `codex exec --json` for long runs so we can persist event streams.
- Use `--output-schema` + `-o <path>` when we need deterministic JSON artifacts.
- Default to **read-only** execution for analysis/review stages; enable edits only in explicitly approved phases.

## CLI contract (high-level)

The CLI should implement (or be designed to implement) these workflows:

- `codex-sdd install`
  - Writes `/prompts:plans` prompt file to `$CODEX_HOME/prompts/plans.md`.
  - Must not touch the repo.

- `codex-sdd init`
  - Creates `AGENTS.md` (this file) and scaffolds `docs/sdd/`.

- `codex-sdd plans`
  - Index repo files deterministically.
  - Run reader agents in parallel (via `codex exec`) that ingest index shards and write digests to `docs/sdd/changes/...`.

- `codex-sdd review` / `codex-sdd tasks`
  - Convert digests/spec deltas into: (a) issues/risks, (b) detailed task lists.
  - Must be file-based and reproducible.

- `codex-sdd approve`
  - Records approval and flips the state gate.

- `codex-sdd worktrees`
  - Creates per-agent worktrees/branches.

- `codex-sdd test-plan`
  - Produces test design artifacts first.
  - Only then generates tests (if approved).

- `codex-sdd select`
  - Aggregates results/metrics across variants and surfaces a recommendation.

- `codex-sdd finalize`
  - Integrates selected variant and archives the change workspace.

### Open decisions (do not hard-code without spec confirmation)

- Whether to add `codex-sdd check` for CI as a first-class command.
- npm / Homebrew distribution strategy (prebuilt vs downloader).
- `finalize` default integration strategy (merge vs cherry-pick).

## CI & “codex-sdd check” guidance (if implemented)

If `codex-sdd check` exists, it should:

- be deterministic and non-interactive,
- avoid requiring model access by default,
- validate repo invariants (state/schema/files), and
- run standard quality gates (format/lint/test) if configured.

## Rust development (this repository)

### Local commands (expected)

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Coverage (optional): `cargo llvm-cov` (tooling-dependent; keep configurable)

### Rust implementation expectations

- Prefer small, composable modules aligned with the architecture (`core`, `analysis`, `codex`, `git`, `quality`, `docs`).
- Treat filesystem and process execution as fallible; return actionable errors.
- Keep outputs stable: deterministic ordering, normalized paths, explicit timestamps/versions.

## Packaging expectations (npm + Homebrew)

- npm packaging must expose a stable executable via `package.json` `bin`.
- If using platform-specific binaries, use npm mechanisms (`optionalDependencies`, `os`, `cpu`) deliberately and document behavior.
- Homebrew distribution should align with bottles/cask norms and avoid repo-side effects at install time.

## References (primary)

- Codex exec (non-interactive): <https://raw.githubusercontent.com/openai/codex/main/docs/exec.md>
- Codex custom prompts: <https://raw.githubusercontent.com/openai/codex/main/docs/prompts.md>
- Codex sandbox & approvals: <https://raw.githubusercontent.com/openai/codex/main/docs/sandbox.md>
- Codex slash commands (built-ins): <https://raw.githubusercontent.com/openai/codex/main/docs/slash_commands.md>
- Slash command prompt folder scanning note: <https://developers.openai.com/codex/guides/slash-commands/>
- AGENTS.md guide (Codex): <https://developers.openai.com/codex/guides/agents-md/>
- npm package.json fields (bin / optionalDependencies / os / cpu): <https://docs.npmjs.com/cli/v8/configuring-npm/package-json/>
- Homebrew bottles: <https://docs.brew.sh/Bottles>
- Git merge options (--no-ff): <https://git-scm.com/docs/merge-options>
- Git cherry-pick (-x): <https://git-scm.com/docs/git-cherry-pick>
- cargo test: <https://doc.rust-lang.org/cargo/commands/cargo-test.html>
- cargo-llvm-cov docs: <https://docs.rs/crate/cargo-llvm-cov/latest/source/docs/cargo-llvm-cov.txt>
