{
  description = "A stunning, real-time terminal system monitor (TUI) for macOS and Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustVersion = "1.95.0";
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Shared build dependencies
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
        ];

        # System-specific libraries
        buildInputs = with pkgs; [
          openssl
          sqlite
          dbus
          lm_sensors
          libpcap
          fontconfig
          libmnl
        ] ++ (lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
          IOKit
          Security
          AppKit
          CoreFoundation
          CoreAudio
          AudioToolbox
        ]));

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            echo "--- 7EMPEST MONITOR DEV ENVIRONMENT ---"
            echo "Rust: $(rustc --version)"
            echo "System: ${system}"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "tempest-monitor";
          version = "0.4.1";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # For Linux DBus/Sensors
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig";

          meta = with pkgs.lib; {
            description = "A stunning terminal system monitor";
            homepage = "https://github.com/7empest462/tempest-monitor";
            license = licenses.mit; # Note: Also includes Commons Clause 1.0
            maintainers = [ "7empest_mac" ];
          };
        };
      }
    );
}
