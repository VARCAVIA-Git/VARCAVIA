# Contributing to VARCAVIA

## Getting Started

```bash
git clone https://github.com/VARCAVIA-Git/VARCAVIA.git
cd VARCAVIA
cargo build --workspace
cargo test --workspace
```

## Before Submitting

Every PR must pass:

```bash
cargo test --workspace     # All tests green
cargo clippy --workspace   # Zero warnings
cargo fmt --all --check    # Formatted
```

## What We Need

### Code
- Bug fixes with regression tests
- Performance improvements with benchmarks
- New CDE pipeline stages
- libp2p networking migration
- ONNX embedding integration

### Documentation
- API usage examples
- Protocol specification clarifications
- Translations

### Testing
- Edge case tests
- Fuzz testing for parsers
- Multi-node integration tests

## Code Style

- Rust: `cargo fmt`, `clippy::all` enforced
- Every public function has a `///` doc comment
- Every module has at least 3 unit tests
- Error handling: `thiserror` for libraries, `anyhow` for binaries
- Commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`

## Architecture Decisions

If your change affects the protocol (dDNA format, ARC consensus, CDE pipeline), open an issue first to discuss. Protocol changes require updating `docs/VERITPROTOCOL.md`.

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0.
