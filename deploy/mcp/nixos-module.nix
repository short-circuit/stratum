# Stratum MCP server — NixOS service module.
#
# Import this module from a NixOS configuration (e.g. a host that serves the
# MCP server) and set the options under `services.stratum-mcp`:
#
#   imports = [ ./deploy/mcp/nixos-module.nix ];
#   services.stratum-mcp = {
#     enable = true;
#     vault = "/var/lib/stratum/vault";
#     bind = "127.0.0.1:8787";
#     tokenFile = "/var/lib/stratum/secrets/mcp-tokens";  # one PAT per line
#     allowedHosts = [ "stratum.example.com" ];
#   };
#
# The module builds `pkm-mcp` from this repository and runs it as a dedicated,
# non-root, hardened systemd unit:
#   - vault mounted read-only (ProtectSystem=strict + ReadOnlyPaths);
#   - auth enabled automatically when `tokenFile` is set (PKM_MCP_TOKEN_FILE);
#   - readiness observable via the embedded `/health` endpoint (§12).
#
# Rollback / restart: `systemctl restart stratum-mcp.service`; to roll back a
# broken build, point `package` back at a previous ref and
# `systemctl start stratum-mcp.service`. Full procedures in deploy/mcp/README.md.

{ config, lib, pkgs, ... }:

let
  cfg = config.services.stratum-mcp;

  # Build pkm-mcp from this repository with the pinned toolchain.
  pkg = pkgs.rustPlatform.buildRustPackage {
    pname = "pkm-mcp";
    version = "0.7.0";
    src = lib.cleanSource ../../.;
    cargoLock.lockFile = ../../Cargo.lock;
    # Only the MCP binary is needed.
    buildPhase = ''
      cargo build --release -p pkm-mcp
    '';
    installPhase = ''
      mkdir -p $out/bin
      cp target/release/pkm-mcp $out/bin/pkm-mcp
    '';
    doCheck = false;
    meta = {
      description = "Stratum MCP server (Model Context Protocol)";
      license = lib.licenses.agpl3Only;
    };
  };

  tokenFileEnv = lib.optional (cfg.tokenFile != null) "PKM_MCP_TOKEN_FILE=${cfg.tokenFile}";
  allowedHostsEnv = lib.optional (cfg.allowedHosts != []) "PKM_MCP_ALLOWED_HOSTS=${lib.concatStringsSep "," cfg.allowedHosts}";
in
{
  options.services.stratum-mcp = {
    enable = lib.mkEnableOption "Stratum MCP server";
    package = lib.mkOption {
      type = lib.types.package;
      default = pkg;
      description = "pkm-mcp package to run (auto-built from the repo by default).";
    };
    vault = lib.mkOption {
      type = lib.types.path;
      description = "Path to the Stratum vault (mounted read-only).";
    };
    bind = lib.mkOption {
      type = lib.types.string;
      default = "127.0.0.1:8787";
      description = "Bind address for the HTTP transport (health at /health).";
    };
    tokenFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "File of PATs (one per line) enabling bearer auth. When null, runs in no-auth (loopback) mode.";
    };
    allowedHosts = lib.mkOption {
      type = lib.types.listOf lib.types.string;
      default = [ ];
      description = "Allowed Host headers (Streamable HTTP). Empty = server default (127.0.0.1,localhost).";
    };
    logLevel = lib.mkOption {
      type = lib.types.string;
      default = "info";
      description = "RUST_LOG filter for tracing.";
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups.stratum-mcp = { };
    users.users.stratum-mcp = {
      isSystemUser = true;
      group = "stratum-mcp";
    };

    systemd.services.stratum-mcp = {
      description = "Stratum MCP server (Model Context Protocol)";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      serviceConfig = {
        User = "stratum-mcp";
        Group = "stratum-mcp";
        ExecStart = "${cfg.package}/bin/pkm-mcp --transport http --bind ${cfg.bind}";
        Restart = "on-failure";
        RestartSec = "5s";
        # Hardening
        ProtectSystem = "strict";
        ReadOnlyPaths = [ cfg.vault ];
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
        MemoryDenyWriteExecute = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        # Environment
        Environment =
          [ "PKM_MCP_VAULT=${cfg.vault}"
            "PKM_MCP_TRANSPORT=http"
            "PKM_MCP_BIND=${cfg.bind}"
            "RUST_LOG=${cfg.logLevel}"
          ] ++ tokenFileEnv ++ allowedHostsEnv;
      };
    };
  };
}
