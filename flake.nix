{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        lib = pkgs.lib;
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Libraries needed at build time and runtime for Tauri (WebKitGTK)
        tauriNativeDeps = with pkgs; [
          webkitgtk_4_1
          libsoup_3
          libayatana-appindicator
          gtk3
          cairo
          gdk-pixbuf
          glib
          openssl
          librsvg
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bun
            rustToolchain
            cargo-tauri
            typescript
            pkg-config
            turso
            exiftool
            spec-kit
          ] ++ tauriNativeDeps;

          env = {
            LD_LIBRARY_PATH = lib.makeLibraryPath tauriNativeDeps;
          };

          shellHook = ''
            echo "Nomen development environment loaded"
            echo "  cargo tauri dev   - Run Tauri in development mode"
            echo "  cargo tauri build - Build production binary"
            echo "  bun run build.ts  - Build frontend only"
            echo "  cargo test        - Run Rust tests (from src-tauri/)"
            echo "  bun test          - Run frontend type tests"
          '';
        };
      }
    );
}
