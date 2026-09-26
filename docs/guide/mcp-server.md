# MCP Server

The Stratum **MCP server** (`pkm-mcp`) exposes your Stratum knowledge base to
external AI clients over the [Model Context Protocol](https://modelcontextprotocol.io)
(MCP). A connected AI assistant can read, write, index, search, and organize
notes in your vault with the same atomic semantics as the desktop editor —
while your notes stay plain Markdown on disk with zero vendor lock-in.

This guide explains how to **set up** the server, **connect an external AI
client**, and **operate it safely**. The wire-level contract (tool schemas,
error codes, data models) is normative in the
[MCP Server Contract](../advanced/mcp.md); this document is the operator's
guide.

!!! info "Architectural reference"
    - **Contract (NORMATIVE):** [MCP Server API Contract](../advanced/mcp.md)
    - **Threat model & security controls:** [ADR-0006](../development/adr/0006-mcp-threat-model.md)
    - **Architecture decision:** [ADR-0005](../development/adr/0005-mcp-server-contract.md)

---

## What the server does

`pkm-mcp` is a standalone binary that reads and writes a Stratum vault's
on-disk data directly — the same SQLite block store, Tantivy search index, and
wiki-link graph the desktop app uses. It speaks the MCP protocol version
`2025-06-18` over two transports:

| Transport | Use case | Authentication |
|-----------|----------|----------------|
| `stdio`   | Same-host, single-client (Claude Desktop, CLI agents, editors) | None (same-user local); optional PAT |
| Streamable HTTP (`http`) | Network clients, remote or multi-client | PAT bearer token (recommended), or none for loopback-only |

There is also a `both` transport that serves stdio and HTTP from one process,
and an operational `GET /health` endpoint for load balancers and monitors.

---

## Prerequisites

- **A Stratum vault** on disk (a directory containing `.pkm/` with
  `blocks.db`). Create one from the desktop app, or initialize an empty one:
  ```bash
  VAULT=/path/to/your/vault
  mkdir -p "$VAULT/.pkm"
  ```
- **The `pkm-mcp` binary.** Two options:

  === "From the repository"

      ```bash
      git clone https://github.com/short-circuit/stratum.git
      cd stratum
      # Build the workspace (or just the MCP crates):
      cargo build -p pkm-mcp
      # The binary is emitted at:
      #   ./target/debug/pkm-mcp
      ```

  === "From source (shipment / release)"

      The `pkm-mcp` binary is built as part of the workspace release pipeline.
      Use the debug or release build matching your platform.

- **Rust 1.75+ / Cargo** if building from source. No Node.js is required for
  the MCP server itself (the frontend is only needed for the desktop app).

!!! note "Linux"
    On Linux you need the standard Rust build toolchain. The MCP server does
    **not** require the Tauri/WebKitGTK system libraries — it is a pure
    networking + local-data binary.

---

## Quick start (stdio)

The `stdio` transport is the fastest way to try the server. It exposes the
same 15 tools as the HTTP transport.

```bash
# Point it at your vault (PKM_MCP_VAULT works too)
./target/debug/pkm-mcp --vault /path/to/your/vault
```

The server reads line-delimited JSON-RPC 2.0 on stdin and writes protocol
frames to stdout (all logs go to stderr, so the stream is never corrupted).
It keeps serving until stdin closes.

To smoke-test it exactly as our CI does:

```bash
python3 crates/pkm-mcp/examples/smoke_stdio.py
```

This launches a real subprocess against a fresh temp vault, performs the MCP
handshake, lists tools, writes a note, reads it back, and searches for it.
Output ends with `[smoke] ALL PASS` on success.

---

## Configuration

Configuration is read from environment variables, with CLI flags overriding
them. The vault path is the only required value.

### Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `PKM_MCP_VAULT` | Path to the Stratum vault to serve | *(required)* |
| `PKM_MCP_BIND` | Bind address for the HTTP transport (`host:port`) | `127.0.0.1:3000` |
| `PKM_MCP_TOKEN` | PAT accepted by the server (plaintext, single token) | *(none)* |
| `PKM_MCP_TOKEN_FILE` | File of PATs, one per line (`token[:scope1,scope2]`) | *(none)* |
| `PKM_MCP_RATE_LIMIT_RPS` | Token-bucket refill rate (requests per second) per key | contract default |
| `PKM_MCP_RATE_LIMIT_BURST` | Token-bucket burst capacity | contract default |
| `PKM_MCP_ALLOWED_HOSTS` | Comma-separated `Host` header allow-list | `127.0.0.1,localhost` |
| `PKM_MCP_NO_AUTH` | If set (any value), disable PAT auth entirely | unset |

### CLI flags

```
pkm-mcp
  --vault <DIR>          Path to the Stratum vault (overrides PKM_MCP_VAULT)
  --transport <TRANS>    stdio | http | both   (default: stdio)
  --bind <ADDR>          Bind address for HTTP (overrides PKM_MCP_BIND)
  --require-auth         Require PAT auth on the HTTP transport
  --help, --version
```

### Examples

Serve over HTTP on a specific port, requiring a PAT:

```bash
export PKM_MCP_VAULT=/path/to/your/vault
export PKM_MCP_TOKEN='AbCdEf…43-char-base64url…'
./target/debug/pkm-mcp --transport http --bind 0.0.0.0:8080 --require-auth
```

Serve both transports in one process:

```bash
PKM_MCP_VAULT=/path/to/your/vault ./target/debug/pkm-mcp --transport both
```

---

## Connecting an external AI client

### 1. Over stdio (local, same-host)

MCP clients that support stdio servers (Claude Desktop, VS Code MCP extensions,
CLI agents such as `claude code`, `mcp`-compatible runners) are configured with
the command that launches the server:

```json
{
  "mcpServers": {
    "stratum": {
      "command": "/absolute/path/to/pkm-mcp",
      "args": ["--vault", "/absolute/path/to/your/vault"]
    }
  }
}
```

!!! note
    Always use an **absolute path** to the vault so the client launches the
    server with the correct working directory.

### 2. Over HTTP (remote / multi-client)

Serve with `--transport http` (optionally `--require-auth`). Point an MCP
client at the base endpoint:

```
http://HOST:PORT/mcp
```

Clients configured for **Streamable HTTP** with a bearer token:

| Setting | Value |
|---------|-------|
| Endpoint | `http://HOST:PORT/mcp` |
| Transport | Streamable HTTP |
| Authorization | `Stratum-MCP <PAT>` (or `Bearer <PAT>`) |
| Protocol version | `2025-06-18` |

!!! tip "Use `--require-auth` in production"
    Over a network, always combine `--require-auth` with a long, random PAT
    (see [Authentication](#authentication--authorization)). Without it, any
    client that can reach the port has full read/write access to your notes.

### 3. Manual verification with `curl`

You can verify a running HTTP server from the shell. First, the handshake:

```bash
# Returns 200 + an mcp-session-id header when authenticated.
curl -i -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Authorization: Stratum-MCP AbCdEf…43-char…' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize",
       "params":{"protocolVersion":"2025-06-18","capabilities":{},
                 "clientInfo":{"name":"manual","version":"1"}}}'
```

Then reuse the returned `mcp-session-id` for tool calls:

```bash
curl -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Authorization: Stratum-MCP AbCdEf…43-char…' \
  -H 'Mcp-Session-Id: <id-from-handshake>' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call",
       "params":{"name":"kb_write_page",
                 "arguments":{"path":"projects/idea.md","content":"prototype the grabber"}}}'
```

The server returns SSE-framed JSON-RPC responses; a successful write returns
the written `NoteDocument` with `"isError": false`.

---

## The 15 MCP tools

All tools require the MCP session to be initialized and are subject to
authentication and scope checks. Output responses are capped at **1 MiB**; the
server never returns absolute filesystem paths in errors. Every tool's binding
JSON Schema is normative in the [contract](../advanced/mcp.md) §5; the exact
argument shapes below match the implementation.

### Read (`kb:read`)

#### `kb_get_page`
Fetch the full content of a note.

- Scope: `kb:read`
- Arguments: `path` (required) — vault-relative markdown path, e.g. `projects/example.md`
- Returns: `NoteDocument` (content, frontmatter, links, tags)
- Errors: `NotFound` (`-32001`) if the note does not exist

```
kb_get_page({ "path": "projects/idea.md" })
```

#### `kb_list_pages`
List notes with pagination.

- Scope: `kb:read`
- Arguments: `limit` (1–1000, default 100), `cursor` (opaque continuation token)
- Returns: a page list, optionally paginated

```
kb_list_pages({ "limit": 50 })
```

#### `kb_index_status`
Return index freshness and coverage.

- Scope: `kb:read`
- Arguments: none
- Returns: index status

```
kb_index_status({})
```

#### `kb_backlinks`
Find backlinks (linked + unlinked mentions) for a note.

- Scope: `kb:read`
- Arguments: `path` (required), `include_unlinked` (boolean, default `true`)
- Returns: backlink references

```
kb_backlinks({ "path": "projects/idea.md" })
```

#### `kb_graph`
Return graph data for a note's subgraph, or the whole graph.

- Scope: `kb:read`
- Arguments: `path` (nullable — when provided, only the subgraph within `depth` hops), `depth` (1–5, default 2)
- Returns: graph nodes and edges

```
kb_graph({ "path": "projects/idea.md", "depth": 2 })
```

#### `kb_resolve_link`
Resolve a `[[wiki-link]]` target to a note path.

- Scope: `kb:read`
- Arguments: `target` (required)
- Returns: the resolved note path

```
kb_resolve_link({ "target": "Grabber" })
```

#### `kb_vault_info`
Return knowledge base metadata and health.

- Scope: `kb:read`
- Arguments: none
- Returns: vault metadata / health

```
kb_vault_info({})
```

### Write (`kb:write`)

#### `kb_write_page`
Create or overwrite a note **atomically**. Reuses the desktop `save_page`
semantics: temp-file + rename for the `.md`, then a SQLite transaction, then
index refresh — so the on-disk file and the index never diverge.

- Scope: `kb:write`
- Arguments: `path` (required), `content` (required, full markdown body),
  `expected_modified` (optional RFC 3339 timestamp guard)
- Returns: the written `NoteDocument`
- Errors: `Conflict` (`-32002`) if `expected_modified` fails the guard

```
kb_write_page({ "path": "projects/idea.md", "content": "prototype the grabber" })
```

#### `kb_delete_page`
Delete a note and its index entries.

- Scope: `kb:write`
- Arguments: `path` (required), `expected_modified` (optional precondition guard)
- Errors: `Conflict` (`-32002`) if the note changed since `expected_modified`

```
kb_delete_page({ "path": "projects/obsolete.md" })
```

### Index (`kb:index`)

#### `kb_reindex`
Reindex the knowledge base.

- Scope: `kb:index`
- Arguments: `mode` — `"incremental"` (default; refreshes changed pages) or
  `"rebuild"` (re-indexes the entire vault, exclusive under advisory lock)
- Returns: index status

```
kb_reindex({ "mode": "incremental" })
```

### Search (`kb:search`)

#### `kb_search`
Full-text search across blocks.

- Scope: `kb:search`
- Arguments: `query` (required, min length 1), `limit` (1–1000, default 20),
  `offset` (≥ 0, default 0)
- Returns: ranked hits

```
kb_search({ "query": "prototype", "limit": 10 })
```

#### `kb_search_by_tag`
Search notes and blocks by tag.

- Scope: `kb:search`
- Arguments: `tag` (required), `limit` (1–1000, default 50)
- Returns: matching notes/blocks

```
kb_search_by_tag({ "tag": "project", "limit": 10 })
```

#### `kb_autocomplete`
Suggest pages, tags, or backlink targets matching a query.

- Scope: `kb:search`
- Arguments: `query` (required), `kind` (`"page"` | `"tag"` | `"backlink"`,
  default `"page"`), `limit` (1–100, default 10)
- Returns: candidate suggestions

```
kb_autocomplete({ "query": "proj", "kind": "page" })
```

### Organize (`kb:organize`)

#### `kb_add_tag`
Add a tag to a note.

- Scope: `kb:organize`
- Arguments: `path` (required), `tag` (required; pattern `^[a-zA-Z0-9_.\-/#]+$`)
- Returns: the updated `NoteDocument`

```
kb_add_tag({ "path": "projects/idea.md", "tag": "grabber" })
```

#### `kb_remove_tag`
Remove a tag from a note.

- Scope: `kb:organize`
- Arguments: `path` (required), `tag` (required; pattern `^[a-zA-Z0-9_.\-/#]+$`)
- Returns: the updated `NoteDocument`

```
kb_remove_tag({ "path": "projects/idea.md", "tag": "grabber" })
```

---

## Authentication & authorization

### Security model

The threat model for the MCP server is **ADR-0006**. The enforceable controls
live in `crates/pkm-mcp-security` and are unit-tested independently of the
transport. In short:

- **PAT (Personal Access Token)** — an opaque bearer token, `Stratum-MCP v1`
  format: **43-character base64url**. This is the default authentication for
  HTTP.
- **Hashed storage** — PATs are held as **SHA-256 hashes only in memory**;
  comparisons are **constant-time**. There is no recoverable plaintext or
  reversible hash anywhere on disk.
- **Scopes** — every token carries a set of scopes; every tool declares the
  exact scope it needs. A token with no scope grants nothing. A `kb:read`-only
  token cannot write, reindex, or reorganize.
- **Rate limiting** — per-key token-bucket limiting. Exceeding it returns
  `RateLimited` (`-32003`) with a `Retry-After`.
- **Path containment** — client-supplied paths are validated against the vault
  root: no traversal (`..`), no absolute paths, no Windows drive prefixes,
  canonical containment below the vault.
- **Size caps** — 2 MiB inbound bodies, 1 MiB per tool response (truncated
  responses are always valid JSON; errors never leak absolute paths).
- **TLS** — terminated at your reverse proxy; the server itself does not
  terminate TLS.

### Scope vocabulary

The server defines eight scopes. Note the mapping from **group** to **actual
tools** (verified against the implementation):

| Scope | Tools granted |
|-------|---------------|
| `kb:read` | `kb_get_page`, `kb_list_pages`, `kb_index_status`, `kb_backlinks`, `kb_graph`, `kb_resolve_link`, `kb_vault_info` |
| `kb:write` | `kb_write_page`, `kb_delete_page` |
| `kb:index` | `kb_reindex` |
| `kb:search` | `kb_search`, `kb_search_by_tag`, `kb_autocomplete` |
| `kb:link` | (reserved) wiki-link graph operations |
| `kb:organize` | `kb_add_tag`, `kb_remove_tag` |
| `kb:admin` | (reserved) operational admin |

### The `Stratum-MCP` bearer format

The HTTP transport accepts the PAT in the `Authorization` header in either form:

```
Authorization: Stratum-MCP <PAT>
Authorization: Bearer <PAT>
```

PATs are 43-character base64url strings (no `=` padding). Generate one with
any CSPRNG:

```bash
# 32 random bytes → 43-char base64url (no padding, no '+' or '/')
openssl rand -base64 32 | tr '+/' '-_' | tr -d '=\n'
```

### Enabling authentication over HTTP

```bash
export PKM_MCP_VAULT=/path/to/your/vault
export PKM_MCP_TOKEN='<43-char-pat>'
./target/debug/pkm-mcp --transport http --require-auth --bind 0.0.0.0:8080
```

!!! important "`--require-auth` is required over the network"
    This is the **verified** behavior: the HTTP transport only enforces the PAT
    when `--require-auth` (or `AuthMode::Pat`) is active. Without it, HTTP
    serves in no-auth mode, which is fine for loopback-only use but unsafe over
    a network. Always pass `--require-auth` when the server is reachable beyond
    localhost.

### Multiple tokens / scoped tokens

The `PKM_MCP_TOKEN_FILE` mechanism (implemented and unit-tested in
`pkm-mcp-security` and `config::load_tokens_from_file`) accepts multiple PATs,
one per line, each optionally carrying scopes:

```
AbCdEf…43-char…:kb:read,kb:search
GhIjKl…43-char…:kb:read,kb:write,kb:index,kb:search,kb:admin
```

!!! warning "Token file wiring in the HTTP transport"
    As of the current build, the HTTP transport (`serve_http`) wires only the
    single `PKM_MCP_TOKEN` value into its validator. The token-file **format
    and validation are fully implemented and tested**, but the HTTP server does
    not yet pass `PKM_MCP_TOKEN_FILE` to the validator. For production HTTP
    deployments with multiple tokens, either:
    - use one token per server instance via `PKM_MCP_TOKEN`, or
    - rely on the token-file JSON format only for the (tested) security layer,
      and watch for this wiring to land in a follow-up.

### OAuth 2.1

OAuth 2.1 is the MCP-standard authorization framework and an *optional*
capability for multi-tenant deployments; the server advertises
`oauth:supported` in its server info when enabled. Single-user and self-hosted
deployments should use PATs.

---

## Rotating credentials

Rotating a compromised or expiring PAT. With the single environment token the
rotation requires a server restart; the steps below keep downtime at zero.

### Scenario A — single env token (requires restart)

1. Generate a new PAT:
   ```bash
   NEW=$(openssl rand -base64 32 | tr '+/' '-_' | tr -d '=\n')
   ```
2. Update `PKM_MCP_TOKEN`: restart the server process with the new value.
3. **In one deploy step if possible**, restart the server and update every
   client's stored token. Because the old token is simply gone, clients using
   it receive `401 Unauthorized` (`-32004`) until updated.
4. Verify the new token works (see [Manual verification](#3-manual-verification-with-curl)).

### Scenario B — multiple tokens (token file)

The token-file **format** supports multiple PATs and is enforced by the
security layer, but (see the warning above) the HTTP transport currently reads
only the single `PKM_MCP_TOKEN`. With the current build the practical rotation
procedure is:

1. Generate the new PAT and set it as `PKM_MCP_TOKEN` on the server.
2. Restart the server (the env token is read once at startup).
3. Update every client to the new token.
4. Once all clients have migrated, the old value is simply no longer valid
   (it is not in the hashed records).

When the multi-token file wiring lands, rotation becomes editing the token
file (add new token, migrate clients, remove old token line) with no restart.

!!! tip "Rotation hygiene"
    - Store PATs in a secret manager (or your shell's env loader), never in
      source control.
    - Issue per-client tokens with **least-privilege scopes**; a read-only
      agent gets `kb:read,kb:search`, a writer gets those plus `kb:write`.
    - The server stores only SHA-256 hashes, so there is no plaintext to
      protect on disk.

---

## Troubleshooting

### The server starts but `tools/list` returns empty / handshake fails
- Confirm the client and server agree on protocol version `2025-06-18`.
- For stdio, make sure nothing else writes to the server's stdout — logs go to
  stderr by design; any tool that injects `println!`-style output into stdout
  corrupts the frame stream. Our own tracing is already routed to stderr.

### `401 Unauthorized` (`-32004`) over HTTP
- The token is wrong, the `Authorization` header is missing/malformed, or the
  server is in no-auth mode.
- Confirm you launched with `--require-auth` **and** set `PKM_MCP_TOKEN`.
- Confirm the exact token matches the server's value (watch for trailing
  newlines when copying from a file).
- Confirm following the *Authorization* format `Stratum-MCP <PAT>`.

### `NotFound` (`-32001`) on a note I know exists
- Paths are vault-relative and must be `.md` paths. `projects/idea` is not the
  same as `projects/idea.md`.
- The note may be outside the vault root; path containment rejects files above
  the vault.

### `RateLimited` (`-32003`)
- You hit the per-key token bucket. Honor `Retry-After` from the error payload
  and back off. Raise limits with `PKM_MCP_RATE_LIMIT_RPS` /
  `PKM_MCP_RATE_LIMIT_BURST` if your client is legitimately bursty.

### `Conflict` (`-32002`) on write
- Another writer (including the desktop app) modified the note after your
  `expected_modified` snapshot. Re-fetch (`kb_get_page`) and retry.

### `ScopesDenied` (`-32005`)
- The token's scopes do not cover the tool you called. Check the token's scope
  list against the tool's required scope (see [The 15 MCP tools](#the-15-mcp-tools)).

### HTTP works with `curl` but not my MCP client
- Some clients do not support the Streamable HTTP transport well; use the
  stdio transport for same-host setups (see [Over stdio](#1-over-stdio-local-same-host)).
- Confirm the client sends `Accept: application/json, text/event-stream` and
  honors the `mcp-session-id` header for follow-up requests.

### I can't reach `/health`
- `/health` is **not** behind auth and is served at `<bind>/health`. If you
  cannot reach it, the process is not listening on that interface — check
  `--bind` and that the port is not already taken.

---

## Developer appendix

### Building & testing the server crate

```bash
cargo build -p pkm-mcp
cargo test -p pkm-mcp          # contract + unit tests (68 tests)
cargo test -p pkm-mcp-security # security controls suite (50+ tests)
cargo clippy -p pkm-mcp --all-targets -- -D warnings
cargo fmt --check
```

### Crate layout

- `crates/pkm-mcp` — the server: `config`, `auth`, `error`, `tools`,
  `kbserver`, `http` (Streamable HTTP + `/health`), `server` (MCP handler),
  `models`.
- `crates/pkm-mcp-security` — transport-agnostic security controls (PAT,
  scope, rate limiter, path containment, input caps, output sanitization).
- `crates/pkm-mcp/examples/smoke_stdio.py` — end-to-end stdio acceptance gate.

### Design invariants

- The MCP write path reuses the **exact atomic semantics** of the desktop
  `save_page`: temp-file + rename → SQLite transaction → index refresh →
  plugin hooks. Output is byte-identical to what the editor produces.
- The server holds **no long-lived write lock**; a per-vault tokio write mutex
  serializes writes (single-writer atomicity).
- All 15 tools, argument names, and schema shapes are **normative** in the
  [contract](../advanced/mcp.md) — changes must update contract + tests together.
