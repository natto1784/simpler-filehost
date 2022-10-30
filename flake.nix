{
  description = "Simpler Filehost";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/release-22.05;
    cargo2nix.url = github:cargo2nix/cargo2nix;
    utils.url = github:numtide/flake-utils;
    rust-overlay.url = github:oxalica/rust-overlay;
  };

  outputs = { self, nixpkgs, utils, cargo2nix, rust-overlay }:
    utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            cargo2nix.overlays.default
            rust-overlay.overlays.default
          ];

          pkgs = import nixpkgs {
            inherit system overlays;
          };

          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

          rustPkgs = pkgs.rustBuilder.makePackageSet {
            packageFun = import ./Cargo.nix;
            rustToolchain = toolchain;
          };

        in
        rec {
          devShell = rustPkgs.workspaceShell {
            packages = [ pkgs.rust-analyzer ];
          };

          packages = rec {
            simpler-filehost = (rustPkgs.workspace.simpler-filehost { }).bin;
            default = simpler-filehost;
            image =
              pkgs.dockerTools.buildImage {
                name = "simpler-filehost";
                config = {
                  Cmd = [ "${simpler-filehost}/bin/simpler-filehost" ];
                  Env = [ "ROCKET_ADDRESS=0.0.0.0" ];
                };
              };
          };
          defaultPackage = packages.default;
        }
      );
}
