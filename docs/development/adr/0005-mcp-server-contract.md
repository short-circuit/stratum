# ADR-0005: MCP Server — Architecture and API Contract

- **Status:** Accepted (normative — implementation tasks build against this)
- **Date:** 2026-09-19
- **Deciders:** architect (API owner), backend-dev, security-auditor, qa-engineer, technical-writer, devops
- **Supersedes:** none
- **Related:** `docs/advanced/mcp.md` (authoritative API contract), crates in the pkm-* workspace, epic "MCP Server" (root t_7a548710)

## Context

Stratum exposes its full knowledge base through native Tauri IPC. External AI
clients cannot call those commands. We want a Model Context Protocol (MCP)
server so that an external AI can read, write, index, search, link, and organize
notes in a Stratum vault. The server must reuse the existing data layer
(SQLite `pkm-block` store, Tantivy `pkm-index` search, `pkm-markdown`
parser/serializer), must be atomic and consistent with the desktop app's
writers, and must be secure when exposed beyond a single user's machine.

This ADR fixes the architecture and the API contract. It is the normative
reference; the implementation card (t_96591769) makes the runtime match it
exactly, and the security (t_4acd5437), QA (t_547e25f1), documentation
(t_28f8e7f1) and deployment (t_af0ce286) cards build against the contract in
`docs/advanced/mcp.md`.

## Decision

1. **Protocol baseline.** The server speaks MCP as defined in the current
   stable specification revision pinned in `docs/advanced/mcp.md` (§1). That
   pin is `2025-06-18` for the *message core* (lifecycle, tools, resources,
   JSON-RPC 2.0 framing) and the **Streamable HTTP** transport (introduced in
   revision `2025-03-26`) as the sole network transport, because the newer
   transport revisions **removed the GET stream endpoint** and changed
   client/server negotiation semantics; pinning avoids silent drift across SDK
   upgrades. `stdio` is also supported for same-host, single-client operation
   (local CLI/editor integration) and MUST be implemented with the same tool
   surface and semantics as the HTTP transport.

2. **SDK.** Implement against the official Rust SDK crate `rmcp`
   (modelcontextprotocol/rust-sdk), which binds to the experimental go-sdk
   transport for tokio. `rmcp` supports both stdio and Streamable HTTP
   transports and is licensed MIT OR Apache-2.0 — license-compatible per the
   dependency policy in `AGENTS.md`. Pin the exact revision in the workspace
   manifest; no hand-rolled protocol layer.

3. **Transport security.** Over HTTP the server MUST be served with TLS
   terminated at the deployment boundary (reverse proxy) unless it binds to a
   loopback-only interface. It exposes exactly one path `<base>/mcp` for MCP
   sessions (Streamable HTTP), plus a non-MCP `<base>/health` compatibility
   endpoint used by load balancers and orchestrators. The stdio transport needs
   no networking configuration and is the default for local only.

4. **Authentication & authorization.** MCP's authorization framework is OAuth
   2.1 (RFC 6749/9700). For Stratum's first release we define two supported
   schemes:

   - **Personal access token (PAT)** — an opaque bearer token (`Stratum-MCP v1`
     format, 43-char base64url, scoped, revocable). This is the default and is
     sufficient for single-tenant/self-hosted use and for all stdio setups.
   - **OAuth 2.1** — implemented per the MCP Authorization spec for
     multi-tenant deployments where an external client must authenticate a
     user without issuing tokens out-of-band. OAuth is an optional capability:
     the server advertises `oauth:supported` in its server info; the nuance is
     in the contract spec (§7).

   Authorization is **inside the server**, per-client, via **scopes** attached
   to the PAT (or to the OAuth token's `scope` claim). Every MCP tool declares
   an exact required scope. Scope vocabulary: `kb:read`, `kb:write`,
   `kb:index`, `kb:search`, `kb:link`, `kb:organize`, and `kb:admin`. A token
   with no scope grants nothing.

5. **Data access.** The MCP server opens the SAME on-disk data the desktop app
   uses: SQLite `blocks.db` under `vault/.pkm/` via `pkm-block::BlockStore`,
   plus the Tantivy index under `vault/.pkm/search` via `pkm-index`. The server
   MUST NOT hold a long-lived write lock; a scoped read-mostly worker is used,
   and every mutation commits inside a SQLite transaction exactly like the
   desktop `save_page`/`save_blocks` commands do (temp-file + rename for the
   `.md`, then transaction for SQLite, then index refresh). The on-disk file is
   always at least as fresh as SQLite, preventing divergence and corruption on
   crash. The write path MUST reuse `pkm_markdown::block_parser` serialization
   and `pkm-markdown` wiki-link extraction so the server's output is byte-identical
   to what the editor produces.

6. **Tool surface.** The MCP tool names, argument names, and input/output
   schemas are NORMATIVE and fixed by `docs/advanced/mcp.md` (§5). Fifteen
   tools are defined, grouped by capability:

   - **Read:** `kb_get_page`, `kb_list_pages`
   - **Write:** `kb_write_page`, `kb_delete_page`
   - **Index:** `kb_reindex`, `kb_index_status`
   - **Search:** `kb_search`, `kb_search_by_tag`, `kb_autocomplete`
   - **Link:** `kb_backlinks`, `kb_graph`, `kb_resolve_link`
   - **Organize:** `kb_add_tag`, `kb_remove_tag`
   - **Meta:** `kb_vault_info`

   Tools directly shadow the existing Tauri command surface (one row per tool
   in the mapping table in `docs/advanced/mcp.md` §6), so the backend
   implementation is a thin adapter over `pkm-block`/`pkm-index`/`pkm-markdown`
   — no duplicated business logic.

7. **Data models.** The MCP input/output schemas reuse the existing DTO shapes
   (`PageDto`, `BlockDto`, `SearchResultDto`, `GraphDataDto`, `BacklinkDto`,
   etc.) from `src-tauri/src/commands/*` wherever the two surfaces overlap, so
   the frontend and the MCP server never observe divergent note/link graph
   shapes. New MCP-only DTOs (e.g. relationship enumeration for `kb_graph`)
   are defined in the contract spec. Note content is always vault-relative
   `path` strings (POSIX separators) and `[[wiki-link]]` targets, consistent
   with the file format spec `docs/advanced/file-format.md`. Response payloads
   are capped at `MCP_RESPONSE_MAX` (1 MiB) with pagination tokens added by the
   contract; clients MUST NOT assume a fixed page size.

8. **Concurrency & consistency.** All read tools run on the same store with a
   per-request check of the SQLite state. Writes are serialized per vault by a
   per-vault write mutex (single writer) so the desktop app and the MCP server —
   and two concurrent MCP writes — cannot corrupt a note. Write conflicts are
   resolved by **last-writer-wins with a precondition guard**: the write tools
   accept an optional `expected_modified` timestamp (RFC 3339) and fail with
   `CONFLICT` (specific error code) when the on-disk note has since changed.
   The desktop app's file watcher is the coordination point: the MCP server tags
   its own saves (`watcher_last_save`-equivalent) so the watcher does not race
   server writes. Index refresh (`kb_reindex`) is exclusive with `kb_write_*`
   via an advisory lock mirroring the desktop `IndexingGuard`.

9. **Rate limiting.** Every PAT-bearer request over HTTP is rate-limited
   per-token with a token bucket: default `MCP_RATE_LIMIT_RPS` burst 60,
   sustained 10 rps (configurable per deployment). OAuth-authenticated requests
   are keyed by token subject when present, else by client id, else by
   authenticated IP. A 429 response uses the JSON-RPC error shape with the
   `Retry-After` header; rate limit state is in-memory by default with an
   optional Redis backend (see §10). sdios and loopback-only instances may
   disable rate limiting.

10. **Error handling.** All MCP errors are returned as JSON-RPC error
    responses with a protocol-conformant code; the canonical error vocabulary
    is `KbErrorCode` (`NotFound`, `Conflict`, `InvalidArg`, `ScopesDenied`,
    `External`...). Full mapping table in the contract spec (§8). Human-readable
    `data` messages explain remediation, with no stack traces or raw paths
    beyond the vault-relative note path; the server MUST NOT leak filesystem
    details or PII in errors.

11. **Observability & ops.** The server logs through the existing `tracing`
    infrastructure (structured, JSON optional). It emits the MCP `logging`
    notification for server-side warnings to connected clients. The
    `/health` endpoint reports read-only availability, store connectivity, and
    index freshness; it is distinct from any MCP session handshake.

## Consequences

- Backend (t_96591769) implements the transport, the 14 tools, and the
  consistency/locking rules as specified here and in `docs/advanced/mcp.md`.
- Security (t_4acd5437) threat-models the PAT/OAuth paths, the scope model, the
  SSRF-resistant http surface and the write-conflict guard, and adds dependency
  supply-chain checks for `rmcp`.
- QA (t_547e25f1) drives the test suite against the fixed tools/schemas and
  the error vocabulary; contract tests MUST validate every tool's JSON Schema.
- Documentation (t_28f8e7f1) writes user-facing guides to match this contract
  and the security model; technical-writer must not invent tools.
- DevOps (t_af0ce286) packages the server (Dockerfile / NixOS module), the
  `/health` probe, secrets for PAT rotation, and CI/CD steps; no deployment-only
  endpoint name changes without a new ADR.
- Contract stability is a release-blocking criterion: tool names, argument
  names, required scopes, and error codes are frozen for the v0.7.x series
  except by a new ADR.

## Status of this ADR

Adopted at design time. Implementation tasks build against the normative
contract in `docs/advanced/mcp.md`; this ADR records the architectural
decisions and is stable unless a decision is revisited by a new ADR.
