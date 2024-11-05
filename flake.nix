{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [fenix.overlays.default];
      };
      rust' = fenix.packages.${system}.fromToolchainFile {
        file = ./rust-toolchain.toml;
        sha256 = "sha256-txii9/4eh2fR+unoHKlPVcGphsHefEiNI+5wLPoCTpA=";
      };
      nativeBuildInputs = with pkgs; [
        # Rust dependencies
        clang
        # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
        llvmPackages.bintools
        # rustup
        rust'

        cargo-watch
        cargo-tarpaulin

        openssl

        pkg-config

        gdal

        # Available only via fenix
        # Useful for integrating this version fo
        rust-analyzer-nightly
      ];
      buildInputs = [];
      allInputs = nativeBuildInputs ++ buildInputs;
      env = with pkgs; {
        LIBCLANG_PATH = lib.makeLibraryPath [
          llvmPackages_latest.libclang.lib
        ];
        RUSTFLAGS = builtins.map (a: ''-L ${a}/lib'') [];
        LD_LIBRARY_PATH = lib.makeLibraryPath allInputs;
        BINGEN_EXTRA_CLANG_ARGS =
          (builtins.map (a: ''-I"${a}/include"'') [
            pkgs.glibc.dev
            pkgs.gdal
          ])
          ++ [
            ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
            ''-I"${pkgs.glib.dev}/include/glib-2.0"''
            ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
          ];
        PKG_CONFIG_PATH =
          lib.strings.concatStringsSep ":"
          (builtins.map (a: ''${a}/lib/pkgconfig'') [
            pkgs.openssl.dev
            pkgs.gdal
          ]);
      };
      shellHook = ''
        export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
        export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
      '';
    in {
      packages.default = fenix.packages.${system}.minimal.toolchain;

      devShells.default = pkgs.mkShell ({
          inherit nativeBuildInputs buildInputs shellHook;
        }
        // env);
    });
}
