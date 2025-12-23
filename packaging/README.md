# Packaging Strategy

## npm

- Meta-package `codex-sdd` provides a JS shim (`bin/codex-sdd.js`).
- Platform-specific binaries are distributed via `optionalDependencies` (e.g. `@codex-sdd/darwin-arm64`).
- npm is the primary distribution path for global installs.
- No auto-download happens at install time; if the binary is missing, install the matching platform package explicitly.

## Homebrew

- Bottle 配布を前提とします。
- macOS: arm64/x86_64
- Linux: arm64/x86_64

## Notes

- `codex-sdd install` を明示的に実行しない限り、プロンプトは変更されません。
