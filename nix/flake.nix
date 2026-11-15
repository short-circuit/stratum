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

        # System libraries for the native gpui shell: X11/Wayland windowing,
        # keyboard layout, font rasterization, Vulkan/EGL rendering, ALSA for
        # text-to-speech playback, GTK + libayatana-appindicator for the tray
        # icon and D-Bus for notifications. The React/WebKitGTK stack was
        # removed at the E12 cutover.
        desktopDeps = with pkgs; [
          gtk3
          glib
          openssl
          dbus
          librsvg
          libayatana-appindicator
          libglvnd
          vulkan-loader
          vulkan-headers
          wayland
          wayland-protocols
          libxkbcommon
          libx11
          libxcb
          libxcb-wm
          libxcursor
          libxrandr
          libxi
        ];

        # Build-time tools
        buildTools = with pkgs; [
          pkg-config
          cmake
          perl
          curl
          fontconfig
          freetype
          zlib
          alsa-lib
          at-spi2-atk
          cairo
          pango
        ];

        # Headless capture/probe tools used by scripts/capture-gpui-*.sh.
        captureTools = with pkgs; [
          xvfb-run
          xdotool
          imagemagick
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          name = "stratum-dev";

          nativeBuildInputs = with pkgs; [
            rustToolchain
            cargo-deny
            cargo-outdated
            git
            rustfmt
            clippy
            jq
          ] ++ desktopDeps ++ buildTools ++ captureTools;

          buildInputs = with pkgs; [
            xdg-utils
          ];

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            # gpui loads the Vulkan loader at runtime; Nix keeps it out of the
            # default library path.
            export LD_LIBRARY_PATH="${pkgs.vulkan-loader}/lib:${pkgs.libglvnd}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
            # The shell renders through software Vulkan on headless machines.
            export LIBGL_ALWAYS_SOFTWARE=''${LIBGL_ALWAYS_SOFTWARE:-1}
            export WINIT_UNIX_BACKEND=''${WINIT_UNIX_BACKEND:-x11}

            echo "  Stratum dev environment"
            echo "  Rust:  $(rustc --version)"
            echo "  WAYLAND: ''${WAYLAND_DISPLAY:-not set}"
            echo ""
            echo "  Commands:"
            echo "    cargo build --workspace          Build all Rust crates"
            echo "    cargo test --workspace           Run all Rust tests"
            echo "    cargo run -p stratum-ui          Run the native shell"
            echo "    STRATUM_FIXTURE=1 cargo run -p stratum-ui   Run it on the fixture vault"
            echo "    ./scripts/capture-gpui-shell.sh  Render the shell screenshot"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "stratum";
          version = "1.0.0";
          src = ../.;

          cargoLock.lockFile = ../Cargo.lock;

          cargoBuildFlags = [ "-p" "stratum-ui" ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            makeWrapper
            wrapGAppsHook3
          ] ++ desktopDeps ++ buildTools;

          buildInputs = with pkgs; [
            xdg-utils
          ];

          # gpui resolves the Vulkan loader at runtime.
          postInstall = ''
            wrapProgram $out/bin/stratum \
              --prefix LD_LIBRARY_PATH : "${pkgs.vulkan-loader}/lib:${pkgs.libglvnd}/lib"
          '';

          doCheck = false;
        };
      });
}
