# Stratum MCP Server — Deployment

This page documents how to deploy and operate the Stratum MCP server
reproducibly, and how its health is observed. The authoritative runtime
contract is [ADR-0005 §12](../development/adr/0005-mcp-server-contract.md) and
the full operations runbook lives in the repository at
`deploy/mcp/README.md` (image build, environment template, rollback and
restart procedures).

## Artifacts

| Artifact | Location |
|---|---|
| Dockerfile (distroless, non-root) | `deploy/mcp/Dockerfile` |
| Environment template | `deploy/mcp/env.template` |
| NixOS service module (systemd, hardened) | `deploy/mcp/nixos-module.nix` |
| Compose example (local smoke) | `deploy/mcp/compose.example.yaml` |
| CI/CD build + deploy | `.github/workflows/mcp.yaml` |
| Container image | `ghcr.io/short-circuit/stratum/mcp` (`latest` + `:<sha>`) |

## Quick start (container)

```bash
docker build -f deploy/mcp/Dockerfile -t ghcr.io/short-circuit/stratum/mcp:latest .
docker run -d --name stratum-mcp \
  --read-only --tmpfs /tmp --cap-drop ALL \
  -p 127.0.0.1:8787:8787 \
  -v /path/to/vault:/vault:ro \
  -e PKM_MCP_VAULT=/vault -e PKM_MCP_TRANSPORT=http -e PKM_MCP_BIND=0.0.0.0:8787 \
  ghcr.io/short-circuit/stratum/mcp:latest
```

The vault must contain `.pkm/blocks.db` (a vault the desktop app has opened).

## Health observability

- `GET /health` — unauthenticated liveness endpoint returning `200
  {"status":"ok","index_fresh":true,...}` when healthy, `503` otherwise. This
  is the authoritative probe for load balancers and orchestrators.
- In-container `HEALTHCHECK` runs `pkm-mcp --health-probe`, which opens the
  block store and checks index freshness directly (distroless has no curl).
- Logs are structured `tracing` records on stderr (`RUST_LOG` filter). The
  server also emits MCP `logging` notifications to connected clients.

For NixOS, a systemd timer probing `/health` (with auto-restart) is shown in
`deploy/mcp/README.md`.

## Rollback and restart

Restart: `systemctl restart stratum-mcp.service` (NixOS) or
`docker restart stratum-mcp`.

Rollback: pin the image to a `:<sha>` tag (never float `latest`). To roll
back, re-run the container with the previous known-good sha tag, verify
`/health` before and after. The server never mutates note files (vault is
mounted read-only), so rolling back an image cannot destroy data; at worst a
stale search index is served, which `/health` reports as `index_fresh:false`.

## Environment variables

See `deploy/mcp/env.template` for the full list. Minimum for a remote
deployment: `PKM_MCP_VAULT`, `PKM_MCP_TRANSPORT=http`, `PKM_MCP_BIND`, and at
least one of `PKM_MCP_TOKEN` / `PKM_MCP_TOKEN_FILE` to enable authentication.
When neither token is set, the server runs in no-auth (loopback) mode.
