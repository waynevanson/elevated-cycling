{
  description = "Nix flake with Crane for a Rust workspace";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [fenix.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};
        rust' = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-KcLWMlLnv9ELJY6l9rWTiRdVkPM6xvar9hk/Ux0PNMQ=";
        };
      in {
        packages.${system} = crane.lib.mkCranePackages {
          inherit system;
          workspaceRoot = ./.;
        };

        # DevShell for the workspace
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust'
            rust-analyzer-nightly
          ];
        };
      }
    );
}
