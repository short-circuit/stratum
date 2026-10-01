# Stratum MCP Server — Deployment & Operations

This directory contains the reproducible deployment and operational
infrastructure for the Stratum MCP server (`crates/pkm-mcp`). It satisfies the
deployment card acceptance criteria:

- **Reproducible deployment** from the repository (Dockerfile + NixOS module).
- **Health is observable** (`/health` endpoint + in-container `--health-probe`).
- **Monitoring/logging** configuration (structured tracing to stderr, health
  probe contract, Prometheus/NixOS timer guidance).
- **Rollback and restart** procedures (below).

Authoritative runtime contract: `docs/advanced/mcp.md` (ADR-0005 §12) and
`crates/pkm-mcp/src/config.rs`.

---

## Components

| Artifact | Purpose |
|---|---|
| `Dockerfile` | Multi-stage, distroless (non-root) production image |
| `.dockerignore` | Keeps the build context minimal (excludes frontend/target) |
| `env.template` | All environment variables, documented with defaults |
| `nixos-module.nix` | Hardened systemd service definition for NixOS hosts |
| `compose.example.yaml` | Local smoke-run wiring (not the production target) |
| `.github/workflows/mcp.yaml` | CI/CD: build → test → image → smoke → deploy gate |

## Runtime contract (what the image does)

- Serves **Streamable HTTP** only. `POST /mcp` is the MCP endpoint;
  `GET /health` is the unauthenticated liveness endpoint (contract §12).
- The vault is served to the server. The server **opens `.pkm/blocks.db`** and
  the search index under `.pkm/search`. It can operate against a **read-only**
  vault **only if `.pkm/blocks.db` already exists** and the search index is
  present; for a fresh/administered vault the server must be able to create
  `.pkm/blocks.db` on first open, so **mount the vault writable** unless you
  are certain the on-disk vault is fully provisioned (see §3a).
- **Auth**: when `PKM_MCP_TOKEN` or `PKM_MCP_TOKEN_FILE` is set, the server
  enforces Bearer PAT auth with per-token scopes (contract §7). When neither
  is set, the transport runs in **no-auth (loopback)** mode — use a reverse
  proxy with `127.0.0.1` bind in production if you rely on that.
- Logs are structured `tracing` records on **stderr** (stdout is reserved for
  the stdio JSON-RPC transport), filterable via `RUST_LOG`.

## 1. Image build (reproducible)

From the repository root (the Dockerfile expects the workspace context):

```bash
docker build -f deploy/mcp/Dockerfile -t ghcr.io/short-circuit/stratum/mcp:latest .
docker build -f deploy/mcp/Dockerfile -t ghcr.io/short-circuit/stratum/mcp:$(git rev-parse --short HEAD) .
```

Reproducibility notes:

- The toolchain is pinned by `rust-toolchain.toml` (the base image version is
  the same stable channel).
- `Cargo.lock` is committed; builds resolve exactly the tested dependency set.
- The image is distroless and non-root (`nonroot:nonroot`), read-only rootfs.

## 2. Environment

Copy `env.template` and set at least:

```bash
PKM_MCP_VAULT=/vault          # required
PKM_MCP_TRANSPORT=http        # image default
PKM_MCP_BIND=0.0.0.0:8787     # required inside Docker
# Auth (set at least one for remote):
PKM_MCP_TOKEN_FILE=/run/secrets/pkm_mcp_tokens
```

Token file format: one PAT per line, `token[:comma-separated-scopes]`. A bare
token gets the default personal scope set (contract §7.2). Never commit the
token file.

## 3. Deployment targets

### 3a. Docker Compose (local smoke)

See `compose.example.yaml`. Requires a host vault (with `.pkm/blocks.db`) and
a token file. Health probe is wired into the container via the binary.

### 3b. GitHub Actions → GHCR

`.github/workflows/mcp.yaml` builds and pushes the image to
`ghcr.io/short-circuit/stratum/mcp` and runs a stdio smoke gate against the
built binary:

- **PR / master**: build, run `cargo test -p pkm-mcp`, run the stdio smoke
  gate (`examples/smoke_stdio.py`), build the image, push `:latest` and
  `:<sha>` to GHCR.
- **Deploy gate**: on push to `master`, the image is pushed and the `deploy`
  job runs `docker run` with a scratch vault and curls `/health`, proving the
  artifact is deployable and its health is observable before merge completes.

To deploy the published image to a live host, pull `:latest` (or pin a
specific sha tag — prefer the sha tag for rollback fidelity) and run:

```bash
docker run -d --name stratum-mcp \
  --read-only --tmpfs /tmp \
  --cap-drop ALL --security-opt no-new-privileges:true \
  -p 127.0.0.1:8787:8787 \
  -v /path/to/vault:/vault \
  -v /path/to/mcp-tokens:/run/secrets/pkm_mcp_tokens:ro \
  -e PKM_MCP_VAULT=/vault -e PKM_MCP_TRANSPORT=http -e PKM_MCP_BIND=0.0.0.0:8787 \
  ghcr.io/short-circuit/stratum/mcp:latest
```

> Mount the vault **writable** (`/vault`, not `/vault:ro`) unless
> `.pkm/blocks.db` is already on disk. Run with `--user $(id -u):$(id -g)` (or
> a UID that owns the vault) so the server can create/lock the database; the
> image's default user is the distroless `nonroot` UID 65532.

### 3c. NixOS service

Import `nixos-module.nix` and set `services.stratum-mcp`. The module builds
`pkm-mcp` from the repo (pinned lock), creates a `stratum-mcp` user, mounts
the vault read-only, and runs a hardened systemd unit bound to
`127.0.0.1:8787` (put a TLS reverse proxy in front). See the file header for
an example.

## 4. Health observability

- **Contract endpoint**: `GET http://127.0.0.1:8787/health` returns `200
  {"status":"ok","index_fresh":true,...}` when healthy; `503` with a
  description when the store or index is degraded. This is the authoritative
  probe for load balancers and orchestrators.
- **No-auth caveat**: `/health` is intentionally unauthenticated (contract
  §12) so monitoring stacks can probe it.
- **In-container probe**: the image's `HEALTHCHECK` runs
  `pkm-mcp --health-probe`, which opens the block store and checks index
  freshness directly (no curl/shell in distroless). It exits non-zero when
  unhealthy.
- **NixOS monitoring**: add a systemd timer to probe `/health` (or scrape via
  your HTTP exporter of choice):

```nix
systemd.timers.stratum-mcp-health = {
  wantedBy = [ "timers.target" ];
  timerConfig = { OnCalendar = "minutely"; };
};
systemd.services.stratum-mcp-health = {
  script = ''
    ${pkgs.curl}/bin/curl -fsS http://127.0.0.1:8787/health >/dev/null \
      || systemctl restart stratum-mcp.service
  '';
};
```

## 5. Logging

- Logs are structured JSON-free `tracing` records on stderr. Docker/systemd
  capture them; ship them to your log collector via the platform's driver.
- Set `RUST_LOG` (e.g. `info`, `mcp=debug`) to tune verbosity. Do not set it
  to `trace` on the stdio transport unless reading stderr only — stdio is the
  protocol channel.
- The server also emits MCP `logging` notifications to connected clients for
  server-side warnings (rate limiting near-boundary, index rebuild
  milestones) — visible to MCP clients, not the log stream.

## 6. Rollback & restart procedures

Restart (pick one):

```bash
# Systemd (NixOS module)
sudo systemctl restart stratum-mcp.service
# Docker
docker restart stratum-mcp
```

Rollback — the image tag is the atomic rollback unit. **Always pin** the image
to a specific git-sha tag (`ghcr.io/short-circuit/stratum/mcp:<sha>`), never
float on `latest` for production.

1. **Identify the last-known-good sha tag.**
   ```bash
   docker manifest inspect ghcr.io/short-circuit/stratum/mcp:<sha>
   ```
2. **Roll the container to that tag** and restart:
   ```bash
   docker run ... ghcr.io/short-circuit/stratum/mcp:<known-good-sha>
   ```
   or, for NixOS, point `services.stratum-mcp.package` back at the previous
   flake/commit and `nixos-rebuild switch`.
3. **Verify health** before and after rollout:
   ```bash
   curl -fsS http://127.0.0.1:8787/health   # expect 200 {"status":"ok",...}
   ```
4. **On failed health** (non-200 or `index_fresh:false`): the orchestrator
   auto-restarts (systemd `Restart=on-failure` / docker `restart:
   unless-stopped`). If still unhealthy, the deploy gate in CI blocks
   promotion — investigate the vault on disk (read-only mount may be stale,
   or the index may need a rebuild via `kb_reindex`).

> **Rollback boundary**: the MCP server is stateless with respect to note
> content — notes live in the vault (mounted read-only) and are not mutated by
> the server. The only server-owned mutable state is the search index under
> `.pkm/search`, which is rebuilt on demand. Rolling back the image therefore
> never destroys data; at worst it serves from a temporarily stale index,
> which `/health` reports as `index_fresh:false`.

## 7. Operational notes / known limits

- The server serves HTTP only in the container. `stdio` transport (for Claude
  Desktop-style same-host clients) runs outside the container from the same
  binary (`cargo run -p pkm-mcp -- --transport stdio` or the built binary).
- Mining index writes from the desktop app while the MCP server runs can make
  the search index temporarily stale; `/health` will report
  `index_fresh:false` until the index catches up — monitor it, don't treat it
  as a crash.
- Rate limits (`PKM_MCP_RATE_LIMIT_BURST` / `RPS`) apply to the HTTP
  transport (contract §9); they are off when `RPS=0`.
