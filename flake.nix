{
  description = "A devShell example";

  inputs = {
    nixpkgs.url      = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        devShell = mkShell rec {
          buildInputs = [
            openssl

            pkgconfig
            exa
            fd

            mold
            clang

            rust-analyzer
            rust-bin.nightly.latest.default
          ];

          nativeBuildInputs = [
            pkg-config
        ];

          APPEND_LIBRARY_PATH = lib.makeLibraryPath [
          ];

          LD_LIBRARY_PATH = APPEND_LIBRARY_PATH;
          PKG_CONFIG_PATH = APPEND_LIBRARY_PATH;

          shellHook = ''
            echo Starting setting up paths...
            alias ls=exa
            alias find=fd
            # export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$APPEND_LIBRARY_PATH"
            # export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$APPEND_LIBRARY_PATH"
            echo Done setting up paths.
          '';
        };
      }
    );
  }

