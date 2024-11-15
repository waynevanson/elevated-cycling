{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    naersk,
    fenix,
    flake-utils,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [fenix.overlays.default];
        };
      in
        with pkgs; let
          # utility functions
          createPkgConfigPath = deps: pkgs.lib.strings.concatStringsSep ":" (builtins.map (a: "${a}/lib/pkgconfig") deps);
          createBindgenExtraClangArgs = deps: (builtins.map (a: ''-I"${a}/include"'') deps);
          createRustFlags = deps: builtins.map (a: ''-L ${a}/lib'') deps;

          rust' = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-txii9/4eh2fR+unoHKlPVcGphsHefEiNI+5wLPoCTpA=";
          };

          naersk' = pkgs.callPackage naersk {
            cargo = rust';
            rustc = rust';
          };

          codebase' = naersk'.buildPackage {
            src = ./.;
          };

          nativeBuildInputs = [
            cargo-watch
            cargo-tarpaulin
            clang
            codebase'
            llvmPackages.bintools
            openssl
            openssl.dev
            pkg-config
            rust'
            rust-analyzer-nightly
          ];
          buildInputs = [
            openssl
            pkg-config
          ];

          environment = {
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
            PKG_CONFIG_PATH = createPkgConfigPath buildInputs;
          };

          shellHook = ''
            export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
            export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
          '';
          common = environment // {inherit nativeBuildInputs buildInputs shellHook;};
          bootstrap = naersk'.buildPackage (common
            // {
              name = "booty";
              version = "0.0.0";
              src = ./.;
              cargoBuildOptions = options:
                options
                ++ [
                  "--bin"
                  "booty"
                ];
            });
        in {
          apps.default =
            common
            // {
              type = "app";
              program = "${bootstrap}/bin/booty";
              RUST_LOG = "info";
            };

          devShells.default = pkgs.mkShell common;
        }
    );
}
