# Stratum MCP Server — API Contract (NORMATIVE)

> **Status: NORMATIVE.** This document is the authoritative contract for the
> Stratum MCP server (ADR-0005). Backend, security, QA, documentation, and
> deployment tasks build against it. The implementation task (t_96591769) must
> make the runtime match this document exactly; QA must validate every tool
> schema against the JSON Schemas in §5.

## Table of Contents

1. [Revision pin](#1-revision-pin-and-normative-facts)
2. [Transports](#2-transports)
3. [Server metadata](#3-server-metadata)
4. [Data models](#4-data-models)
5. [Tools (JSON Schema)](#5-tools--json-schema)
6. [Mapping to existing storage and command surface](#6-mapping-to-existing-storage-and-command-surface)
7. [Authentication & authorization](#7-authentication--authorization)
8. [Error handling & error codes](#8-error-handling--error-codes)
9. [Rate limiting](#9-rate-limiting)
10. [Concurrency, atomicity & consistency](#10-concurrency-atomicity--consistency)
11. [Pagination, limits & payload caps](#11-pagination-limits--payload-caps)
12. [Operational endpoints & observability](#12-operational-endpoints--observability)
13. [Integration plan for backend developers](#13-integration-plan-for-backend-developers)

---

## 1. Revision pin and normative facts

- MCP message core: **revision `2025-06-18`** (lifecycle `initialize`/`initialized`,
  `tools/list`, `tools/call`, `resources/*`, JSON-RPC 2.0 framing).
- Transport: **Streamable HTTP** (introduced in revision `2025-03-26`), served
  over a single HTTP(S) endpoint. Newer transport revisions removed the GET
  stream endpoint and changed negotiation; the pinned revision keeps behavior
  stable for this contract's lifetime.
- Second transport (same tool surface): **stdio**, for same-host single-client
  use.
- Official Rust SDK: `rmcp` (modelcontextprotocol/rust-sdk), MIT OR Apache-2.0.
- Protocol versions announced in `initialize`:
  `protocolVersion: "2025-06-18"`, `capabilities: { tools: {listChanged: false} }`.
- All examples in this document are illustrative; schemas in §5 are binding.

## 2. Transports

### 2.1 Streamable HTTP

- Endpoint: `<base>/mcp`.
- Request method: `POST` with `Content-Type: application/json`. Body is a
  JSON-RPC 2.0 request object (or a batch of them).
- Response: JSON-RPC 2.0 response object. The request is a batch of
  JSON-RPC messages and the server processes them in order, responding in order.
- One **session** per initiating request; `Mcp-Session-Id` header is used for
  follow-up requests when the server requires it. When authentication is
  enabled, the server issues a session id and requires it on subsequent
  requests belonging to the same logical session.
- No GET stream endpoint (per pinned revision). Clients connect using the MCP
  SDK and do not rely on `/mcp` GET.
- `Content-Type` of responses is `application/json`.

### 2.2 stdio

- The stdio transport is line-delimited JSON-RPC 2.0 over stdin/stdout.
- Configuration for a client would be the single command that launches the
  server binary with `--stdio` and the vault path. Same tools, same semantics.

## 3. Server metadata

Server info returned in `initialize`:

- `name`: `stratum-mcp`
- `version`: MUST match the workspace `pkm-*` version (currently `0.7.x`).
- `capabilities.tools.listChanged`: `false` (tools are static for the pinned
  revision).

## 4. Data models

Reuse existing DTO shapes where the two surfaces overlap (see §6). The
following MCP-specific models are defined here. All are illustrative examples;
schemas in §5 are binding.

### 4.1 `NoteDocument` — the full content model of a note

```json
{
  "path": "projects/example.md",
  "slug": "projects/example",
  "title": "Example note",
  "content": "# Raw markdown body\n\n- bullet one\n- bullet two\n\nA [[wiki-link]] to another note.",
  "frontmatter": {
    "title": "Example note",
    "created": "2026-01-01T10:00:00Z",
    "modified": "2026-09-19T08:00:00Z",
    "tags": ["project", "research"]
  },
  "links": [
    {"target": "Another Note", "block_id": "1920e42d-...", "position": 42}
  ],
  "tags": ["project", "research"],
  "block_count": 3,
  "modified_at": "2026-09-19T08:00:00Z"
}
```

All `path` values are vault-relative, use `/` separators, and `slug` is the
page path with `.md` removed. `links` are the `[[wiki-link]]` targets
extracted via `pkm-markdown::linker::extract_links` (same extraction the
graph and backlinks use).

### 4.2 `Block` — matches the editor's block DTO

```json
{
  "id": "9bf2d85f-2c5a-4b5e-9d3e-00a1f00b1c22",
  "content": "Some block text",
  "parent_id": null,
  "left_id": null,
  "properties": {"#tag": null},
  "marker": null,
  "priority": null,
  "collapsed": false,
  "heading_level": null
}
```

`marker` ∈ {"TODO","DOING","DONE","WAITING","CANCELLED"} ∪ null; `priority`
∈ {"A","B","C"} ∪ null.

### 4.3 `SearchHit` — matches `SearchResultDto`

```json
{
  "block_id": "9bf2d85f-...",
  "content": "…",
  "page_path": "projects/example.md",
  "snippet": "…",
  "score": 0.87
}
```

### 4.4 `Backlink` — matches `BacklinkDto`

```json
{
  "source_id": "…",
  "source_page": "other/note.md",
  "context": "…",
  "is_linked": true
}
```

### 4.5 `GraphData` — matches `GraphDataDto`

```json
{
  "nodes": [{"id": "…", "title": "…", "path": "…", "tags": [], "degree": 2}],
  "edges": [{"source": "…", "target": "…", "label": null}],
  "node_count": 0,
  "edge_count": 0,
  "vault_path": "/absolute/vault"
}
```

### 4.6 `VaultInfo` — matches `VaultInfo` command output

```json
{
  "vault_path": "/absolute/vault",
  "page_count": 0,
  "block_count": 0,
  "index_fresh": true,
  "last_indexed_at": null
}
```

### 4.7 `TagOperationResult` — returned by tag mutation tools

```json
{
  "path": "projects/example.md",
  "success": true,
  "tag": "project",
  "action": "add",
  "applied": 3
}
```

### 4.8 `IndexStatus` — returned by `kb_index_status`

```json
{
  "index_fresh": true,
  "indexed_pages": 12,
  "total_pages": 12,
  "indexed_blocks": 240,
  "last_indexed_at": "2026-09-19T08:00:00Z"
}
```

All timestamp values are RFC 3339 UTC strings. All `id` values for blocks are
UUID v4 strings. Response payloads are bounded per §11.

## 5. Tools  (JSON Schema)

Each tool is registered under the given name in `tools/list`. The `inputSchema`
for each tool is **binding**; the backend MUST reject inputs that fail schema
validation with `InvalidArg`.

### 5.1 `kb_get_page` — read a single note

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Read a note by its vault-relative path",
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Vault-relative markdown path, e.g. 'projects/example.md'."
    }
  },
  "required": ["path"],
  "additionalProperties": false
}
```

Response: a `NoteDocument`. On a missing note, returns error `NotFound`
(`-32001`).

### 5.2 `kb_list_pages` — enumerate notes

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "List notes in the knowledge base with pagination",
  "type": "object",
  "properties": {
    "limit": {
      "type": "integer",
      "minimum": 1,
      "maximum": 1000,
      "default": 100
    },
    "cursor": {
      "type": "string",
      "description": "Opaque continuation token from a previous response."
    }
  },
  "required": [],
  "additionalProperties": false
}
```

Response:

```json
{
  "pages": [
    {"path": "projects/example.md", "slug": "projects/example", "title": "Example note", "block_count": 3, "modified_at": "2026-09-19T08:00:00Z"}
  ],
  "next_cursor": null
}
```

`next_cursor` is absent (or null) when there are no more pages.

### 5.3 `kb_write_page` — create or overwrite a note

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Write a note atomically",
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Vault-relative markdown path, e.g. 'projects/example.md'."
    },
    "content": {
      "type": "string",
      "description": "Full markdown body (including any frontmatter to preserve/insert)."
    },
    "expected_modified": {
      "type": ["string", "null"],
      "description": "RFC 3339 timestamp; if provided and the on-disk note has a newer modified time, the write fails with CONFLICT (-32002)."
    }
  },
  "required": ["path", "content"],
  "additionalProperties": false
}
```

Response: the written `NoteDocument` (freshly parsed from disk). The write path
is: serialize/validate → atomic temp-file+rename of `.md` → SQLite transaction
(`delete_blocks_by_page` + `insert_block` per block + `upsert_page`) →
index refresh → watcher exclusion → plugin `onSave`/`onLink` dispatch (same as
desktop `save_page`). On `expected_modified` mismatch, **no mutation occurs**
and `CONFLICT` is returned.

### 5.4 `kb_delete_page` — delete a note

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Delete a note and its index entries",
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Vault-relative markdown path to delete."
    },
    "expected_modified": {
      "type": ["string", "null"],
      "description": "Optional precondition guard; fails with CONFLICT if the note changed since this timestamp."
    }
  },
  "required": ["path"],
  "additionalProperties": false
}
```

Response: `{}` on success. On a missing note, `NotFound` (`-32001`).

### 5.5 `kb_reindex` — rebuild/incremental index

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Reindex the knowledge base (rebuild or incremental)",
  "type": "object",
  "properties": {
    "mode": {
      "type": "string",
      "enum": ["incremental", "rebuild"],
      "default": "incremental",
      "description": "incremental refreshes changed pages; rebuild re-indexes the entire vault (exclusive under advisory lock)."
    }
  },
  "required": [],
  "additionalProperties": false
}
```

Response: `IndexStatus`.

### 5.6 `kb_index_status` — inspect index health

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Return index freshness and coverage",
  "type": "object",
  "properties": {},
  "additionalProperties": false
}
```

Response: `IndexStatus`.

### 5.7 `kb_search` — full-text search

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Full-text search across blocks",
  "type": "object",
  "properties": {
    "query": {"type": "string", "minLength": 1},
    "limit": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 20},
    "offset": {"type": "integer", "minimum": 0, "default": 0}
  },
  "required": ["query"],
  "additionalProperties": false
}
```

Response:

```json
{
  "results": [
    {"block_id": "…", "content": "…", "page_path": "…", "snippet": "…", "score": 0.87}
  ],
  "total": 1,
  "next_offset": null
}
```

`total` is the total matched count; `next_offset` is null when the end of
results is reached.

### 5.8 `kb_search_by_tag` — tag search

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Search notes and blocks by tag",
  "type": "object",
  "properties": {
    "tag": {"type": "string", "minLength": 1},
    "limit": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 50}
  },
  "required": ["tag"],
  "additionalProperties": false
}
```

Response: same shape as `kb_search` (only blocks whose page has the tag in
frontmatter or whose content contains the literal `#tag`).

### 5.9 `kb_autocomplete` — completion suggestions

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Suggest pages, tags, or backlink targets matching a query",
  "type": "object",
  "properties": {
    "query": {"type": "string", "minLength": 1},
    "kind": {
      "type": "string",
      "enum": ["page", "tag", "backlink"],
      "default": "page"
    },
    "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
  },
  "required": ["query"],
  "additionalProperties": false
}
```

Response:

```json
{
  "items": [
    {"text": "Example note", "kind": "page", "detail": "projects/example.md"}
  ]
}
```

### 5.10 `kb_backlinks` — inbound links & mentions

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Find backlinks (linked + unlinked mentions) for a note",
  "type": "object",
  "properties": {
    "path": {"type": "string"},
    "include_unlinked": {"type": "boolean", "default": true}
  },
  "required": ["path"],
  "additionalProperties": false
}
```

Response:

```json
{
  "backlinks": [
    {"source_id": "…", "source_page": "other/note.md", "context": "…", "is_linked": true}
  ]
}
```

### 5.11 `kb_graph` — graph neighborhood

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Return graph data for a note (or the whole graph)",
  "type": "object",
  "properties": {
    "path": {
      "type": ["string", "null"],
      "description": "When provided, only the subgraph of nodes within 'depth' hops of this note. When null, the whole graph."
    },
    "depth": {"type": "integer", "minimum": 1, "maximum": 5, "default": 2}
  },
  "required": [],
  "additionalProperties": false
}
```

Response: `GraphData`.

### 5.12 `kb_resolve_link` — resolve a wiki-link target

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Resolve a [[wiki-link]] target to a note path",
  "type": "object",
  "properties": {
    "target": {"type": "string", "minLength": 1}
  },
  "required": ["target"],
  "additionalProperties": false
}
```

Response:

```json
{
  "resolved": {"path": "projects/example.md", "slug": "projects/example", "title": "Example note"},
  "unresolved": false
}
```

If the target does not resolve to any note, `resolved` is `null` and
`unresolved` is `true`.

### 5.13 `kb_add_tag` — organize: add a tag

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Add a tag to a note",
  "type": "object",
  "properties": {
    "path": {"type": "string"},
    "tag": {"type": "string", "minLength": 1, "pattern": "^[a-zA-Z0-9_.\\-/#]+$"}
  },
  "required": ["path", "tag"],
  "additionalProperties": false
}
```

Response: `TagOperationResult`.

Tag insertion semantics: add `tag` to the note's frontmatter `tags` array
(idempotent). If an inline `#tag` already exists in the content, the operation
counts the note as already tagged and leaves it unchanged (success).

### 5.14 `kb_remove_tag` — organize: remove a tag

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Remove a tag from a note",
  "type": "object",
  "properties": {
    "path": {"type": "string"},
    "tag": {"type": "string", "minLength": 1, "pattern": "^[a-zA-Z0-9_.\\-/#]+$"}
  },
  "required": ["path", "tag"],
  "additionalProperties": false
}
```

Response: `TagOperationResult`.

Tag removal semantics: remove `tag` from the note's frontmatter `tags` array
and remove any literal `#tag` occurrences from block content, then re-serialize
and write like `kb_write_page`. Idempotent: if neither exists, the operation
succeeds with `applied: 0`.

### 5.15 `kb_vault_info` — meta

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Return knowledge base metadata and health",
  "type": "object",
  "properties": {},
  "additionalProperties": false
}
```

Response: `VaultInfo`.

## 6. Mapping to existing storage and command surface

The MCP tools are thin adapters over the existing crates and Tauri command
surface. Reuse the exact data access used by the desktop app; do not duplicate
business logic.

| MCP tool | Required scope | Storage / command surface it reuses | Backing crate |
|---|---|---|---|
| `kb_get_page` | `kb:read` | `commands::page::open_page`, `BlockStore::get_blocks_by_page`, `get_page` | `pkm-block`, `pkm-markdown` |
| `kb_list_pages` | `kb:read` | `commands::page::list_pages` (exclude `.git/`) | `pkm-block` |
| `kb_write_page` | `kb:write` | `commands::page::save_page` (atomic file + SQLite txn + index refresh) | `pkm-block`, `pkm-index`, `pkm-markdown` |
| `kb_delete_page` | `kb:write` | `commands::page::delete_page` | `pkm-block` |
| `kb_reindex` | `kb:index` | `commands::search::rebuild_search_index` / `reindex_page` | `pkm-index` |
| `kb_index_status` | `kb:read` | `IndexEngine` freshness check over `vault/.pkm/search` | `pkm-index` |
| `kb_search` | `kb:search` | `commands::search::search_blocks` | `pkm-index` |
| `kb_search_by_tag` | `kb:search` | `commands::search::search_by_tag` | `pkm-index` |
| `kb_autocomplete` | `kb:search` | `commands::search::autocomplete` | `pkm-index`, `pkm-block` |
| `kb_backlinks` | `kb:read` | `commands::search::get_page_backlinks` | `pkm-index` |
| `kb_graph` | `kb:read` | `commands::graph::get_graph_data` (subgraph via depth) | `pkm-index`, `pkm-block` |
| `kb_resolve_link` | `kb:read` | `commands::graph::resolve_link_target` / `BlockStore::resolve_link_target_path` | `pkm-block` |
| `kb_add_tag` | `kb:organize` | frontmatter `tags` mutation → `kb_write_page` path | `pkm-markdown`, `pkm-block` |
| `kb_remove_tag` | `kb:organize` | frontmatter + inline `#tag` removal → `kb_write_page` path | `pkm-markdown`, `pkm-block` |
| `kb_vault_info` | `kb:read` | `commands::vault::get_vault_info` | `pkm-core`, `pkm-block` |

Path safety: all user-supplied `path`/`target` values MUST pass vault-root
containment checks equivalent to `resolve_safe_path` /
`resolve_safe_write_path` in `src-tauri/src/commands/page.rs` (canonicalize, then
verify `starts_with(vault_root)`), and the server MUST reject traversal.

### 6.1 How link & organize operations map to MCP tools

- **Link creation** is performed by writing `[[wiki-link]]` syntax in note
  content through `kb_write_page` (scope `kb:write`). The server reuses
  `pkm-markdown::linker::extract_links` on save to index the new edges, exactly
  as the desktop save path does. There is no separate "create link" tool.
- **Link read-out** (inbound links, mentions, neighborhood, target resolution)
  is served by `kb_backlinks`, `kb_graph`, and `kb_resolve_link`, all under
  scope `kb:read`.
- **Organize** (tags) is served by `kb_add_tag` / `kb_remove_tag` under scope
  `kb:organize`; both reuse the `kb_write_page` write path internally.
- The `kb:link` scope is **reserved** for future standalone link-management
  tools; it is granted by default to personal PATs but no current tool requires
  it. A tool that requires it must be introduced by a new ADR.

## 7. Authentication & authorization

### 7.1 Modes

- **Local/stdio:** no authentication required (vault path is passed by the
  client at launch). The server assumes the invoking user trusts the vault.
- **Remote (Streamable HTTP):**
  - **PAT (required capability for v0.7.x).** An opaque bearer token issued by
    the server (or by a separate operator flow). Format `Stratum-MCP <token>`
    in the `Authorization` header; token is 43-char base64url (256-bit
    entropy), stored hashed (SHA-256) server-side, revocable at any time.
    Token creation/revocation is an operator admin function; the contract does
    not define an HTTP admin API for it (see DevOps card).
  - **OAuth 2.1 (optional capability).** Implement per the MCP Authorization
    spec when the deployment requires interactive multi-tenant login. The
    server advertises OAuth support in server info (`authorization` metadata);
    when unadvertised or unconfigured, clients MUST use PAT or skip
    authentication (local-only deployments).

### 7.2 Scopes

Every tool requires exactly one scope (from the mapping table in §6; scope
`kb:admin` exists for operator/admin endpoints and rate-limit config). A token
grants a set of scopes; a request whose tool requires a scope the client does
not hold is rejected with `ScopesDenied` (`-32005`) and `HTTP 403`.

Default token scopes for a "personal" PAT: all of `kb:read`, `kb:write`,
`kb:index`, `kb:search`, `kb:link`, `kb:organize`. The `kb:admin` scope is not
granted by default.

### 7.3 Boundaries

- Read tools never mutate; write tools never mutate outside the vault.
- Scope checks happen **before** any storage access.
- The `/health` endpoint never requires authentication.
- The MCP `initialize` handshake does not require authentication; tool calendar
  calls do.
- OAuth token validation MUST verify the `aud` claim (if present) and the
  `scope` claim; unknown scopes are ignored.

### 7.4 Credential rotation

- PATs are hashed at rest; rotation is create-new + revoke-old (multi-key
  support: multiple valid hashes at once). Rotation procedure is documented by
  the technical-writer card and implemented by the security card.

## 8. Error handling & error codes

All errors are JSON-RPC 2.0 error responses:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Note not found: projects/example.md",
    "data": {
      "kind": "NotFound",
      "note_path": "projects/example.md",
      "hint": "Create it with kb_write_page, or check the path."
    }
  }
}
```

### 8.1 Canonical code vocabulary

`-32000` through `-32099` are the JSON-RPC implementation-defined range (MCP
reserves this range; nothing in the MCP spec further standardizes subcodes, so
the values below are internal but stable within Stratum).

| Code | `kind` | Meaning | Typical HTTP |
|---|---|---|---|
| `-32700` | ParseError | malformed JSON | 400 |
| `-32600` | InvalidRequest | invalid JSON-RPC request | 400 |
| `-32601` | MethodNotFound | unknown method / tool | 400 |
| `-32602` | InvalidArgs | JSON Schema validation failed (`InvalidArg`) | 400 |
| `-32603` | Internal | unexpected server error | 500 |
| `-32000` | External | downstream store/search error (DB, index) | 500 |
| `-32001` | NotFound | note/path/tag not found | 404 |
| `-32002` | Conflict | precondition guard failed (`expected_modified`) | 409 |
| `-32003` | RateLimited | rate limit exceeded (also HTTP 429) | 429 |
| `-32004` | Ssid | SSRF guard denied target (future) | — |
| `-32005` | ScopesDenied | missing required scope | 403 |
| `-32006` | VaultLocked | index rebuild exclusive lock held, retry later | 423 |

### 8.2 Rules

- Never return stack traces. `message` is human-readable; `data.hint` suggests
  remediation.
- Never leak absolute filesystem paths outside the vault; use vault-relative
  `note_path`.
- A `NotFound` on a read tool never falls back to fabricating content.
- Transient failures (`VaultLocked`, `RateLimited`) MUST include
  `Retry-After` and/or a `data.retry_after_seconds` value.

## 9. Rate limiting

- Applies to authenticated HTTP tool calls only (not to `initialize`, not to
  stdio, not to `/health`).
- Algorithm: per-key token bucket (burst = `MCP_RATE_LIMIT_BURST` default 60,
  refill = `MCP_RATE_LIMIT_RPS` default 10/s).
- Key: token subject when the client is authenticated with a PAT/OAuth subject;
  else client-id; else authenticated source IP.
- On exceed: JSON-RPC error `RateLimited` (`-32003`) with `Retry-After` header
  (seconds to refill one token) and HTTP 429.
- Limits must be configurable per deployment and per scope tier
  (e.g. `kb:write` can be throttled tighter than `kb:read`).
- Storage of counters: in-memory by default (single process); Redis optional
  for multi-instance deployments.

## 10. Concurrency, atomicity & consistency

- **Single writer per vault.** The MCP server serializes write tools per vault
  with an in-process (per-process) write mutex. When the desktop app and the
  MCP server target the same vault, the on-disk `.md` + SQLite transactional
  write (same as `save_page`) keeps the two consistent; the file watcher uses a
  server-side "own save" exclusion equivalent to `watcher_last_save` so the
  watcher does not echo server writes back into a diverging state.
- **Atomicity.** Every write is: validate input → build markdown → temp-file +
  rename (`.md`) → SQLite transaction → index refresh → plugin hooks. If the
  SQLite transaction fails, the renamed file is removed so no divergence
  persists (mirrors `save_blocks` rollback).
- **Preconditions.** `expected_modified` guards prevent blind last-writer-wins.
  The comparison uses the on-disk note's `modified` (RFC 3339); a mismatch is
  `Conflict` with zero mutation.
- **Index exclusivity.** `kb_reindex` (rebuild) takes the advisory index lock
  (`IndexingGuard`-equivalent). Concurrent writers wait for the rebuild; the
  server returns `VaultLocked` only if the wait budget is exceeded.
- **Reads during writes.** Reads are served from the store without a long-lived
  write lock; SQLite WAL or equivalent must be used so readers do not block on
  writers.

## 11. Pagination, limits & payload caps

- `MCP_RESPONSE_MAX` = 1 MiB per tool response. A response approaching the cap
  is truncated along the listed result set with a `next_cursor` /
  `next_offset` continuation, never a hard failure.
- `MCP_BODY_MAX` = 2 MiB on inbound request bodies (well above the 1 MiB
  response cap; guards against oversized write payloads).
- `MCP_PAGE_LIMIT_MAX` = 1000 (already enforced in the schemas above).
- `MCP_PATH_MAX` = 4096 chars.
- `MCP_TOKEN_MAX_SCORE_MATCHES` = 5000 for search pagination.

## 12. Operational endpoints & observability

- `GET <base>/health` — **no auth**. Returns `200` with
  `{"status":"ok","store":"ok","index_fresh":true,"version":"…"}` when the
  store opens and the index is not corrupt; otherwise `503` with a description.
  Used by orchestrators/load balancers and the DevOps card.
- `POST <base>/mcp` — the only MCP session endpoint.
- Logs: `tracing`-based, structured; the server emits MCP `logging`
  notifications to connected clients for server-side warnings (rate limiting
  near-boundary, index rebuild milestones).
- Metrics: optional Prometheus exposition on a separate port, gated to
  operators; not part of the MCP contract.

## 13. Integration plan for backend developers

The backend card (t_96591769) SHALL implement in this order:

1. **Workspace wiring.** Add the MCP crate to the workspace (`crates/pkm-mcp`),
   depending on `pkm-core`, `pkm-block`, `pkm-markdown`, `pkm-index`,
   `pkm-query` as needed; add `rmcp` to `Cargo.toml` workspace dependencies
   (verify AGPL compatibility — `rmcp` is MIT OR Apache-2.0). Update the
   `AGENTS.md` crate/dependency tables and the docs nav per the repo's
   Documentation Sync Rules.
2. **Config module.** Read vault path, transport, TLS/bind, auth mode, rate
   limits from config/env (see DevOps card for env template). Fail fast at
   startup if the vault is not a valid Stratum vault.
3. **Adapters over existing commands.** Implement each tool as a thin function
   calling the same crate functions the Tauri command handlers call; reuse the
   DTO types. Do not re-implement parsing/indexing.
4. **Path safety + validation.** Enforce §6 path checks and every tool's JSON
   Schema validation before dispatch.
5. **Auth + scopes.** PAT validation (hashed lookup), scope check per tool,
   optional OAuth 2.1 behind a feature flag. Every tool maps through §7.2.
6. **Rate limits.** Token bucket middle layer over the HTTP transport (§9).
7. **Error mapping.** Translate crate errors to the §8 vocabulary; never let a
   raw error cross the boundary.
8. **Transport.** Register the server with `rmcp` (stdio + Streamable HTTP)
   and the `/health` endpoint.
9. **Logging + hooks.** Wire tracing; replicate plugin `onSave`/`onLink`
   dispatch and the watcher exclusion for writes.
10. **Tests.** Unit tests per tool adapter; contract tests validating each
    tool's schema and error codes; integration tests against a temp vault using
    `tempfile` and `BlockStore` (see QA card for the fuller strategy; the 80%
    handler-coverage gate is QA's).

Acceptance for the backend card: all 15 tools functional against a real temp
vault via an MCP client, missing notes handled per §8, atomic writes verified,
schemas validate.
