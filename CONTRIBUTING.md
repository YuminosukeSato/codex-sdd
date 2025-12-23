# Contributing

Thanks for your interest in contributing to codex-sdd. Contributions are welcome.

## Before you start
- Please open an issue to discuss significant changes.
- Keep changes focused and small when possible.

## Development setup
```bash
git clone <repo_url>
cd codex-sdd
cargo build
```

## Quality checks
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Pull requests
- Clearly describe what changed and why.
- Update documentation when behavior changes.
- Add tests where it makes sense.
- Ensure CI passes.

## License
By contributing, you agree that your contributions will be licensed under the MIT License.
