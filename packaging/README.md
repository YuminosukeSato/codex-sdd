# Packaging Strategy

## npm

- Meta-package `codex-sdd` provides a JS shim (`bin/codex-sdd.js`).
- Platform-specific binaries are distributed via `optionalDependencies` (e.g. `@codex-sdd/darwin-arm64`).
- npm is the primary distribution path for global installs.
- No auto-download happens at install time; if the binary is missing, install the matching platform package explicitly.
- Platform packages live under `packaging/npm-platforms/<platform>`.

### Release automation

- GitHub Actions publishes npm packages on release tags.
- The workflow builds per-platform binaries, publishes platform packages first, then the meta package.
- Version numbers in Cargo and npm package.json files must match the tag `vX.Y.Z`.
- `NPM_TOKEN` is required as a GitHub Actions secret.

## Homebrew

Homebrew support is still in progress.

## Notes

- `codex-sdd install` does not run automatically; it must be executed explicitly.
