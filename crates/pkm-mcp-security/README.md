# pkm-mcp-security

Security controls for the Stratum MCP server (ADR-0005 contract,
`docs/advanced/mcp.md`). Self-contained, transport-agnostic, and unit-tested so
the controls are verifiable independent of the server's in-flux transport code.

## Modules

| Module | Contract § | Responsibility |
|--------|-----------|----------------|
| `pat`   | 7.1, 7.4 | PAT format validation, SHA-256 hashed storage, constant-time compare, bearer parsing, generation |
| `scope` | 7.2     | Scope vocabulary, `require_scope` before storage access |
| `ratelimit` | 9   | Per-key token-bucket limiter; `Allowed`/`Limited` decisions with `Retry-After` math |
| `path`  | 6        | Vault-relative path validation; traversal/absolute/Windows-drive rejection; canonical + lexical containment |
| `input` | 11       | Size caps (body, query, block content, block count) |
| `output`| 8.2, 11   | Vault-relative error scrubbing; valid-JSON response truncation under `MCP_RESPONSE_MAX` |
| `error` | 8.1      | JSON-RPC error-code constants + `SecurityError` → contract code mapping |

## Usage

The `pkm-mcp` server binds these controls into its dispatch/HTTP layer. The
intended call order per HTTP tool request:

```
parse Authorization → pat::parse_authorization
validate token      → pat::TokenValidator::authenticate
check scope         → scope::require_scope
rate limit          → ratelimit::check
validate args       → input::* (schema validation is rmcp-side)
contain path        → path::validate_vault_path / canonical_contained
dispatch            → tool handler
sanitize output     → output::*
```

## Security properties

- PATs are stored as SHA-256 hashes only; the hash compare is constant-time.
- Scopes are exact-match (no prefix affinity); no scope ⇒ nothing.
- The token bucket is per-key and capped so a long idle period cannot overflow.
- Paths are rejected if absolute, `..`-escaping, NUL-containing, or overlong;
  Windows drive-prefixed paths are treated as absolute.
- Truncated responses are always valid JSON; errors never leak absolute paths.

## Tests

`cargo test -p pkm-mcp-security` — 50+ tests covering PAT format/hashing/rotation,
scope decisions, burst/refill/clamp/isolation of the rate limiter, path
traversal cases, input caps, and output sanitization/truncation invariants.
`cargo clippy -p pkm-mcp-security --all-targets -- -D warnings` and
`cargo fmt --check` are clean.

This crate is AGPL-3.0-only like the rest of Stratum.
