# Contributing

## Getting Started

1. Clone the repository
2. `cargo build --workspace`
3. `cargo test --workspace`

## Development Workflow

- All changes go through pull requests
- Tests must pass before merging
- Follow Rust idioms: `cargo clippy`, `cargo fmt`

## RFC Process

Major features require an RFC. Open an issue with the RFC template:

- **Motivation**: Why this feature?
- **Design**: How does it work?
- **Alternatives**: What else was considered?
- **Migration**: Impact on existing users

## Testing Guidelines

- Unit tests alongside implementation (same file, `#[cfg(test)]` module)
- Integration tests in `tests/` directory for public API
- Use `tempfile` for filesystem tests
- Aim for >80% coverage on core crates

## Code Review

All PRs need:
1. Clean CI (build + test + clippy)
2. At least one reviewer
3. Documentation updates if API changes
