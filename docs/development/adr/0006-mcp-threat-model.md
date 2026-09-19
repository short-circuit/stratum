# ADR-0006 — MCP Server Threat Model & Security Controls

- **Status:** accepted
- **Deciders:** security-auditor, backend-dev, architect (API owner), devops, qa-engineer
- **Date:** 2026-09-19
- **Related:** `docs/advanced/mcp.md` (authoritative API contract), ADR-0005 (MCP
  server contract), `crates/pkm-mcp` (server), `crates/pkm-mcp-security` (controls)
- **Replaces:** n/a

## Context

The MCP server exposes a read/write window into the Stratum knowledge base to
external AI clients over two transports (stdio, Streamable HTTP). This ADR is
the **threat model** for that exposure. It:

1. documents the trust boundaries and assets;
2. applies STRIDE per tool and per transport;
3. records the controls that MUST be enforced (and where each is enforced);
4. records the vulnerability assessment and supply-chain posture;
5. defines the acceptance criteria for the security review and test suite.

The controller source of truth for wire behavior is ADR-0005. Where this ADR
uses words like "MUST" the requirement originates from the contract and is
mandatory; the "control" column names the enforcing component.

## Scope

In scope: the `pkm-mcp` server crate, its two transports, the 15 `kb_*` tools,
PAT/scope auth, rate limiting, and the write path (atomic save semantics).

Out of scope (owned elsewhere): TLS termination (reverse proxy / devops card),
OAuth2.1 identity provider integration, the desktop app's own command layer.

## Trust boundaries

| # | Boundary | Sender at risk | Trust | Controls |
|---|----------|----------------|-------|----------|
| B1 | External AI client → HTTP transport | knowledge base (read/write) | Low (public/untrusted network) | PAT auth, scopes, TLS (proxy), rate limit, size caps |
| B2 | External AI client → stdio transport | same-host notes | Medium (same-user local) | implicit no-auth or optional PAT, filesystem perms |
| B3 | MCP tool handler → BlockStore/Index | notes integrity | Internal | single-writer mutex, atomic write, validation |
| B4 | MCP tool handler → markdown serializer | on-disk `.md` fidelity | Internal | no-loss serialization round-trip |
| B5 | Client-provided `path`/`target` → filesystem | arbitrary files under vault | Low | path containment, canonical check |
| B6 | Search/link output → client | note content disclosure | Internal | scopes gate reads; sanitized errors |

**Assets:** note files (`.md`), block store (SQLite), search index, wiki-link graph,
tags, PAT records (hashes).

## STRIDE analysis

### Per transport

| Threat | HTTP (B1) | stdio (B2) |
|--------|-----------|------------|
| Spoofing | Forged bearer token → attacker impersonates a valid client | Same-user local IPC; spoofing only if another process on host can attach |
| Tampering | Modified request body → schema validation + size caps reject | Payloads over stdio are under client control; same trust |
| Repudiation | No meaningful; audit logs of tool calls to `/health` in-memory | n/a |
| Info disclosure | Unauthenticated read of private notes if PAT not required → MUST require PAT for HTTP unless loopback-only | stdio is same-user; acceptable |
| DoS | Unbounded bodies, unbounded results, request flood → rate limiter + body/response caps | Local; less risk, but long-running sessions still bounded |
| Elevation | Scope escalation: token with `kb:read` performing writes → scope checks before dispatch | If auth disabled, all scopes implicitly granted |

### Per tool class

Write tools (`kb_write_page`, `kb_delete_page`):

| Threat | Vector | Control |
|--------|--------|---------|
| Tampering / corruption | Atomic write interrupted mid-write leaves truncated `.md` | temp-file + rename (reuse of desktop `save_page` path), single-writer mutex |
| Write to wrong path | `path` with `../` or absolute | path containment (§path) |
| Lost update (two clients) | concurrent writes clobber | single-writer mutex + `expected_modified` conflict guard |
| Oversize payload | `content`/blocks too large | body cap + per-field caps |
| Write outside vault | `path` escapes root | canonical containment |
| Deleting read-only area | delete of vault root itself | path containment rejects empty/collapses |

Index tools (`kb_reindex`):

| Threat | Vector | Control |
|--------|--------|---------|
| DoS via lock | reindex holds exclusive lock indefinitely | bounded exclusive lock; non-blocking status |
| Tampering index | concurrent reindex + write | lock ordering, atomic swap |

Search/link tools (`kb_search`, `kb_search_by_tag`, `kb_autocomplete`,
`kb_backlinks`, `kb_graph`, `kb_resolve_link`):

| Threat | Vector | Control |
|--------|--------|---------|
| Info disclosure | unauthenticated search returns private note contents | search requires `kb:search`/`kb:read` scope (no unauthenticated default over HTTP) |
| Query injection | very long query / regex | query length cap, bounded token budget |
| Resource exhaustion | huge result set | response cap (1 MiB) + truncation to valid JSON |
| Path leak in errors | absolute fs path in error payload | output sanitizer (`to_vault_relative`) |

Read tools (`kb_get_page`, `kb_list_pages`, `kb_index_status`, `kb_vault_info`):

| Threat | Vector | Control |
|--------|--------|---------|
| Info disclosure | reading arbitrary paths outside vault | path containment |
| Traversal | `../../etc/passwd` | containment + normalized path |

Organize/link tools (`kb_add_tag`, `kb_remove_tag`, `kb_resolve_link`):

| Threat | Vector | Control |
|--------|--------|---------|
| Tag injection | tag strings with control chars/newlines corrupting frontmatter | input length + charset validation |
| Link target injection | `[[target]]` with `]`/nul breaking parsing | validation |

## Controls and enforcement mapping

Each control below has a concrete owner. The `pkm-mcp-security` crate exists so
these controls are unit-tested independent of the transport; the `pkm-mcp`
server binds them. Where the server crate is not yet present in this worktree,
the control crate + its tests are the reference acceptance payload.

| Control | Contract § | Enforced by | Tested by |
|---------|-----------|-------------|-----------|
| PAT format validation (43-char base64url) | 7.1 | `pkm-mcp-security::pat` | `pat::*` |
| PAT hashed storage (SHA-256), constant-time compare | 7.4 | `pkm-mcp-security::pat` | `pat::*` |
| Scope vocabulary + required-scope per tool | 7.2 | `pkm-mcp-security::scope` | `scope::*` |
| Scope check before storage access | 7.2 | `pkm-mcp` dispatch | (server) |
| Token-bucket rate limiting per key | 9 | `pkm-mcp-security::ratelimit` | `ratelimit::*` |
| Path containment (no traversal, no absolute) | 6 | `pkm-mcp-security::path` | `path::*` |
| Canonical containment below vault root | 6 | `pkm-mcp-security::path` | `path::*` |
| Body/field size caps | 11 | `pkm-mcp-security::input` | `input::*` |
| Response cap + valid-JSON truncation | 8.2, 11 | `pkm-mcp-security::output` | `output::*` |
| Error payload sanitization (no absolute paths) | 8.2 | `pkm-mcp-security::output` | `output::*` |
| Single-writer atomic write | 8/10 | `pkm-mcp` (reuse save_page) | (server) |
| Conflict guard (`expected_modified`) | 8 | `pkm-mcp` | (server) |
| TLS at proxy | 3 | devops | — |
| Rate-limit honeypot diagnostics | 9 | ratelimit `tracked_keys`/`reset` | `ratelimit::*` |

## Supply-chain posture

- Rust: `cargo audit` runs in CI (job `security-audit`) with a pinned baseline
  in `.cargo/audit.toml`. Baseline covers the pre-existing transitive cluster
  (wasmtime / cap-std / unic-* / glib / lru / chacha20); any **new** advisory
  fails the pipeline.
- npm: `npm audit` runs report-only in the same job; 3 known production `high`
  findings are pinned by transitive `@blocknote` / `@excalidraw` deps and
  tracked for remediation via Dependabot.
- Dependabot (`.github/dependabot.yml`) opens bounded weekly update PRs for
  cargo, npm and GitHub Actions so the baseline shrinks over time.
- `pkm-mcp-security`'s own dependencies (sha2, base64, constant_time_eq,
  serde_json, thiserror, serde) are current and carry no advisories.

## Vulnerability assessment (workspace baseline, 2026-09-19)

Audit of the full workspace `Cargo.lock` (846 crates) before baselining:
24 findings (misc severities) + 13 `unsound`/`yanked` warnings. None affect the
MCP security crate. Highest-impact clusters:

| Cluster | Crates | Notes |
|---------|--------|-------|
| wasmtime / wasi | `wasmtime*` (~18 advisories) | WASM plugin runtime; advisories are config-specific; pinned by `pkm-plugin` (wasmtime 26). Remediation: bump wasmtime when plugin ABI allows. |
| cap-std / rustix | via wasmtime-wasi | coboundary of the above |
| lru | `LruCache` unsoundness | used by index internals; benign in practice |
| unic-* | `unic-char-*` | unicode-derived crates; low real-world impact |
| glib | `VariantStrIter` unsoundness | Linux gtk path only |
| chacha20 | yanked | used by `cpal`/audio path; benign |
| h2 | moderate | http/2 crate; used by reqwest |

None of these are reachable from attacker-controlled MCP inputs in a way that
rises above the documented severity. The strict `cargo audit` gate ensures new
findings are caught.

## Known npm findings (after `npm audit fix`)

| Package | Severity | Path | Remediation |
|---------|----------|------|-------------|
| `@tiptap/core` | high | via `@blocknote/core` | bump blocknote when upstream releases |
| `lodash-es` | high | via `@excalidraw/*` → chevrotain | bump excalidraw |
| `nanoid` | high | via `@excalidraw/*` | bump excalidraw |

## Security review checklist (acceptance)

- [ ] Auth: every HTTP tool call without a valid PAT is rejected (unless
      loopback-only no-auth mode explicitly configured).
- [ ] Scopes: a request with `kb:read` only cannot perform a write or a
      reindex; denied → `ScopesDenied` (-32005) before storage access.
- [ ] Rate limit: per-client bucket; exceeded → `RateLimited` (-32003) with
      `Retry-After`.
- [ ] Path: `../` escapes, absolute paths, Windows drive paths all rejected as
      `InvalidArgs`.
- [ ] Write: cannot escape vault root; atomic temp+rename; concurrent writes
      serialized; `expected_modified` conflicts → `Conflict` (-32002).
- [ ] Output: no absolute fs paths in error payloads; responses ≤ 1 MiB and
      always valid JSON when truncated.
- [ ] Supply chain: `cargo audit` clean (baseline pinned) in CI.
- [ ] The `pkm-mcp-security` unit suite (≥50 tests) passes; server-side
      integration suite (QA card t_547e25f1) covers the 15 tools end-to-end.
