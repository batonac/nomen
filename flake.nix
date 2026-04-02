{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    bun2nix = {
      url = "github:nix-community/bun2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, bun2nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ bun2nix.overlays.default ];
        };
        lib = pkgs.lib;
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          pname = "electrobun-nomen";
          version = "0.1.0";

          src = self;

          # Fetch bun dependencies using bun2nix
          bunDeps = pkgs.bun2nix.fetchBunDeps {
            bunNix = ./bun.nix;
          };

          nativeBuildInputs = with pkgs; [
            bun
            typescript
            patchelf  # For patching electrobun binary
          ] ++ [
            pkgs.bun2nix.hook  # Sets up node_modules from pre-fetched cache
          ];

          buildInputs = with pkgs; [
            webkitgtk_4_1
            libsoup_3
            libayatana-appindicator
            gtk3
            cairo
            gdk-pixbuf
          ];

          env = {
            NIX_LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
              glib
              webkitgtk_4_1
              libsoup_3
              libayatana-appindicator
              gtk3
              cairo
              gdk-pixbuf
              stdenv.cc.cc.lib
            ]);
            NIX_LD = lib.fileContents "${pkgs.stdenv.cc}/nix-support/dynamic-linker";
            LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
              glib
              webkitgtk_4_1
              libsoup_3
              libayatana-appindicator
              gtk3
              cairo
              gdk-pixbuf
              stdenv.cc.cc.lib
            ]);
          };

          buildPhase = ''
            # bun2nix.hook has already set up node_modules from pre-fetched cache
            bun run build.ts
          '';

          installPhase = ''
            mkdir -p $out/{lib,bin}
            cp -r dist $out/lib/app
            cp -r bundle/* $out/bin/ 2>/dev/null || true
          '';
        };

        # Dev shell for local development
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bun
            typescript
            turso
            exiftool
            spec-kit
            webkitgtk_4_1
            patchelf  # For patching electrobun binary
          ];

          nativeBuildInputs = [
            pkgs.bun2nix.hook  # Sets up node_modules from pre-fetched cache
            pkgs.bun2nix       # For regenerating bun.nix when needed
          ];

          bunDeps = pkgs.bun2nix.fetchBunDeps {
            bunNix = ./bun.nix;
          };

          env = {
            NIX_LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
              glib
              webkitgtk_4_1
              libsoup_3
              libayatana-appindicator
              gtk3
              cairo
              gdk-pixbuf
              stdenv.cc.cc.lib
            ]);
            NIX_LD = lib.fileContents "${pkgs.stdenv.cc}/nix-support/dynamic-linker";
            LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
              glib
              webkitgtk_4_1
              libsoup_3
              libayatana-appindicator
              gtk3
              cairo
              gdk-pixbuf
              stdenv.cc.cc.lib
            ]);
          };

          shellHook = ''
            echo "Nomen development environment loaded"
            echo "  bun run build    - Build web assets with Bun and package with Electrobun"
            echo "  bun run start    - Build and start Electrobun"
            echo "  bun run dev      - Watch mode for development"
            echo "  bun run test     - Run tests"

            # Patch electrobun binary to use Nix's dynamic linker
            # (electrobun downloads its CLI and needs to work in NixOS)
            NIX_INTERP=$(${pkgs.patchelf}/bin/patchelf --print-interpreter "$(which bun)" 2>/dev/null)
            if [ -n "$NIX_INTERP" ]; then
              for bin in \
                "$PWD/node_modules/electrobun/bin/electrobun"; do
                if [ -f "$bin" ] && ! ${pkgs.patchelf}/bin/patchelf --print-interpreter "$bin" 2>/dev/null | grep -q nix; then
                  ${pkgs.patchelf}/bin/patchelf --set-interpreter "$NIX_INTERP" "$bin" 2>/dev/null || true
                fi
              done
            fi
          '';
        };
      }
    );
}

