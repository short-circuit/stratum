# syntax=docker/dockerfile:1.7
#
# Stratum MCP Server — reproducible production image.
#
# Single-stage release build of the `pkm-mcp` binary, copied into a minimal
# non-root distroless runtime. The server is a self-contained Rust binary and
# needs no shell, curl or package manager at runtime.
#
# Build (from repository root):
#   docker build -f deploy/mcp/Dockerfile -t ghcr.io/short-circuit/stratum/mcp:latest .
#
# Runtime contract:
#   - HTTP transport only (env PKM_MCP_TRANSPORT=http), binds 0.0.0.0:8787.
#   - Vault mounted read-only at PKM_MCP_VAULT (default /vault).
#   - PAT via PKM_MCP_TOKEN or PKM_MCP_TOKEN_FILE; when neither is set the
#     server runs in no-auth mode (loopback / trusted network only).
#   - Health: the binary itself supports `--health-probe` (GET /health) so the
#     distroless image has a working HEALTHCHECK without curl/wget.
#
# See deploy/mcp/README.md and deploy/mcp/env.template for the full contract.

# ---- builder: pinned toolchain (rust-toolchain.toml is source of truth) ----
# image tag: Debian bookworm (glibc 2.36). MUST match (or be OLDER than) the
# glibc of the distroless runtime below — building on trixie (glibc 2.41) links
# against symbols that the bookworm runtime cannot resolve (GLIBC_2.38+).
FROM rust:1.96-bookworm AS builder
WORKDIR /build

# Copy the workspace source. .dockerignore keeps node_modules/target/.git out.
COPY . .

# Compile the MCP server and its crate graph only (release).
RUN cargo build --release -p pkm-mcp \
    && mkdir -p /out \
    && cp target/release/pkm-mcp /out/pkm-mcp

# ---- runtime: minimal distroless ----
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

USER nonroot:nonroot
WORKDIR /srv

COPY --from=builder /out/pkm-mcp /usr/local/bin/pkm-mcp

ENV PKM_MCP_VAULT=/vault \
    PKM_MCP_BIND=0.0.0.0:8787 \
    PKM_MCP_TRANSPORT=http

EXPOSE 8787

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/usr/local/bin/pkm-mcp", "--health-probe"]

ENTRYPOINT ["/usr/local/bin/pkm-mcp"]
