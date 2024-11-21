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
          name = "workspace";
          src = ./.;
          cargoClippyOptions = _: ["-A clippy::all"];
        };
        nativeBuildInputs = with pkgs; [
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
        buildInputs = with pkgs; [
          openssl
          pkg-config
          docker
          docker-compose
          gnutar
          xz
        ];

        environment = {
          LIBCLANG_PATH = pkgs.lib.makeLibraryPath [
            pkgs.llvmPackages_latest.libclang.lib
          ];
          RUSTFLAGS = createRustFlags [];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (nativeBuildInputs ++ buildInputs);
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

        elevated-cycling = {
          cli = naersk'.buildPackage {
            name = "elevated-cycling-cli";
            version = "0.0.0";
            src = ./.;
            cargoBuildArgs = args: args ++ ["--bin elevated-cycling"];
          };
        };

        range-split = naersk'.buildPackage {
          name = "range-split";
          version = "0.0.0";
          src = ./.;
          cargoBuildArgs = args: args ++ ["--bin range-split"];
        };

        main = pkgs.dockerTools.buildImage {
          name = "elevated-cycling";
          tag = "latest";

          contents = [elevated-cycling.cli];

          config = {
            Cmd = ["RUST_LOG=info" "/bin/elevated-cycling"];
          };
        };
        # wrapper = pkgs.writeWrapperShellScriptBin "docker-compose"
        # TODO: write an application that is the entrypoint to docker
        # Prod would usually mean prod databases and in the cloud.
        # We're never really going to prod.
        # Our dev is the cargo watch -x run --bin elevated-cycling <fiules>
      in {
        packages.range-split = elevated-cycling.range-split;
        devShells.default = pkgs.mkShell common;
      }
    );
}
