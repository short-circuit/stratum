# Contributing to Stratum

The full contribution guide lives at
[`docs/contributing.md`](docs/contributing.md) (rendered:
https://short-circuit.github.io/stratum/docs/contributing/).

Quick summary:

- **License:** AGPL-3.0-only. Your contributions are licensed under the
  same license.
- **Start:** fork + branch off `master`, follow
  [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
  (`feat(scope): ...`, `fix(scope): ...`, `docs: ...`).
- **Verify before PR:**
  ```bash
  nix develop ./nix        # or install deps manually (Rust 1.75+, Node 22+)
  cargo build --workspace
  cargo test --workspace
  npm install
  npm run test
  npm run lint
  npm run build
  ```
- **Docs are mandatory** — every code change must update the relevant
  section of `AGENTS.md` / `docs/guide/` / `docs/development/`. See the
  "Documentation rules" table in `docs/contributing.md`.
- **Plugins:** WASM plugins are submitted to the
  [stratum-plugins](https://github.com/short-circuit/stratum-plugins)
  repository (see `PLUGIN_TEMPLATE.md`).

For the full guide (branch naming, commit style, testing, E2E, docs rules,
RFC process, dependency policy): [docs/contributing.md](docs/contributing.md).
