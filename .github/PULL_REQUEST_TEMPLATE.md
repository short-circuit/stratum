<!--
  Stratum PR template.
  Replace the bracketed placeholders. Delete the section that does not apply
  (bug fix vs feature). Keep the checklist honest.
-->

## What does this change do?

<!-- One or two sentences. What is the user-visible or developer-visible
     outcome of this PR? -->

## What's the motivation / context?

<!-- Bug it fixes (issue # or task id), feature request, or design decision.
     Reference the kanban task id if applicable: (t_<task-id>). -->

## Type of change

- [ ] Bug fix (non-breaking, fixes an issue)
- [ ] New feature (non-breaking, adds functionality)
- [ ] Breaking change (API or behavior)
- [ ] Docs / tooling / chore (no functional change)

## Verification

<!-- What did you run, and what was the result? Be specific. -->

- [ ] `cargo fmt --check` (Rust formatting)
- [ ] `cargo clippy --workspace -- -D warnings` (lint gate in CI)
- [ ] `cargo test --workspace` (or scoped: `cargo test -p <crate>`)
- [ ] `npm run test` (frontend unit/component tests)
- [ ] `npm run lint` (ESLint)
- [ ] `npm run build` (`tsc -b && vite build`)
- [ ] `mkdocs build -f docs/mkdocs.yml --strict` (if docs changed)
- [ ] `bash scripts/mcp-coverage-gate.sh` (if MCP crates changed)

## Documentation

- [ ] `AGENTS.md` updated (component table / dependency graph / workspace
      layout, if applicable)
- [ ] `docs/guide/` or `docs/development/` updated (mandatory per AGENTS.md)
- [ ] Not needed (internal change — explain why)

## Checklist

- [ ] Branch is up to date with `master`
- [ ] Commit history uses Conventional Commits
- [ ] Screenshots / evidence images contain no personal info (PII gate)
- [ ] No transient artifacts tracked (`.hermes/` only; see AGENTS.md)
