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

        rust'
        rust-analyzer-nightly
        cargo-watch
        cargo-tarpaulin

        openssl
        openssl.dev
        pkg-config
      ];
      buildInputs = with pkgs; [
        openssl
        openssl.dev
        pkg-config
      ];
      createPkgConfigPath = deps: pkgs.lib.strings.concatStringsSep ":" (builtins.map (a: "${a}/lib/pkgconfig") deps);
      createBindgenExtraClangArgs = deps: (builtins.map (a: ''-I"${a}/include"'') deps);
      createRustFlags = deps: builtins.map (a: ''-L ${a}/lib'') deps;
      env = with pkgs; {
        LIBCLANG_PATH = lib.makeLibraryPath [
          llvmPackages_latest.libclang.lib
        ];
        RUSTFLAGS = createRustFlags [];
        LD_LIBRARY_PATH = lib.makeLibraryPath (nativeBuildInputs ++ buildInputs);
        BINGEN_EXTRA_CLANG_ARGS =
          createBindgenExtraClangArgs (with pkgs; [glibc.dev])
          ++ [
            ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
            ''-I"${pkgs.glib.dev}/include/glib-2.0"''
            ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
          ];
        PKG_CONFIG_PATH = createPkgConfigPath (with pkgs; [openssl openssl.dev pkg-config]);
      };
      shellHook = ''
        export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
        export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
      '';
      main = pkgs.rustPlatform.buildRustPackage (env
        // {
          inherit nativeBuildInputs buildInputs shellHook;
          pname = "bootay";
          version = "0.0.0";
          src = ./.;
          cargoBuildOptions = [
            "--bin"
            "booty"
          ];
          cargoHash = "sha256-RNkuRHTmNIBx00VWMgYCG1QFpqv+dBUAjU2xWPLkW6g=";
        });
    in {
      packages.default = main;
      apps.default =
        env
        // {
          inherit nativeBuildInputs buildInputs shellHook;
          type = "app";
          program = "${main}/bin/booty";
          RUST_LOG = "info";
        };

      devShells.default = pkgs.mkShell (env
        // {
          inherit nativeBuildInputs buildInputs shellHook;
        });
    });
}
