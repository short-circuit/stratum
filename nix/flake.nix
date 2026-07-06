{
  description = "Stratum PKM — privacy-first personal knowledge management system";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;

        # System libraries for Tauri v2 — Nix auto-includes .dev outputs
        tauriDeps = with pkgs; [
          webkitgtk_4_1
          gtk3
          glib
          libsoup_3
          openssl
          dbus
          librsvg
          libayatana-appindicator
          libglvnd
          wayland
        ];

        # Build-time tools
        buildTools = with pkgs; [
          pkg-config
          cmake
          perl
          curl
          fontconfig
          zlib
          alsa-lib
          at-spi2-atk
          cairo
          pango
          libxkbcommon
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          name = "stratum-dev";

          nativeBuildInputs = with pkgs; [
            rustToolchain
            cargo-tauri
            cargo-deny
            cargo-outdated
            nodejs_22
            git
            rustfmt
            clippy
          ] ++ tauriDeps ++ buildTools;

          buildInputs = with pkgs; [
            xdg-utils
          ];

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"

            # Nix stores compiled GSettings schemas under
            # $out/share/gsettings-schemas/$name/glib-2.0/schemas/.
            # GSETTINGS_SCHEMAS_PATH is Nix-specific; standard GTK uses
            # GSETTINGS_SCHEMA_DIR.  Without this, webkit2gtk cannot read
            # org.gnome.desktop.interface.text-scaling-factor, causing
            # devicePixelRatio to return −1/96 (WebKit bug #287811).
            schemas=""
            for dir in ''${GSETTINGS_SCHEMAS_PATH//:/ }; do
              schemas="$schemas''${schemas:+:}$dir/glib-2.0/schemas"
            done
            export GSETTINGS_SCHEMA_DIR="$schemas"

            echo "  Stratum dev environment"
            echo "  Rust:  $(rustc --version)"
            echo "  Node:  $(node --version)"
            echo "  npm:   $(npm --version)"
            echo "  WAYLAND: ''${WAYLAND_DISPLAY:-not set}"
            echo ""
            echo "  Commands:"
            echo "    cargo build --workspace     Build all Rust crates"
            echo "    cargo test --workspace      Run all Rust tests"
            echo "    cargo tauri dev             Run Tauri dev server"
            echo "    npm run dev                 Run Vite dev server only"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "stratum";
          version = "0.5.0";
          src = ../.;

          cargoLock.lockFile = ../Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            wrapGAppsHook3
            cargo-tauri
            nodejs_22
          ] ++ tauriDeps ++ buildTools;

          buildInputs = with pkgs; [
            xdg-utils
          ];

          preBuild = ''
            npm ci --ignore-scripts
            npm run build
            export TAURI_SKIP_DEVSERVER_CHECK=true
          '';

          buildPhase = ''
            cargo tauri build --bundles deb
          '';

          doCheck = false;
        };
      });
}
