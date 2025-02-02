{
  description = "screenshot gui that wraps cli tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default-linux";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        toolchain_dev = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        platform_dev = pkgs.makeRustPlatform {
          rustc = toolchain_dev;
          cargo = toolchain_dev;
        };
        shots = pkgs.callPackage ./shots.nix { };
      in
      {
        packages = {
          default = shots;
          inherit shots;
          inherit (shots) shots-unwrapped;
        };

        apps.default = flake-utils.lib.mkApp { drv = shots; };

        devShells.default = pkgs.mkShell (
          {
            inputsFrom = [
              (shots.override { rustPlatform = platform_dev; }).shots-unwrapped
            ];
            buildInputs = [
              pkgs.cargo-nextest
              pkgs.cargo-audit
              pkgs.rust-bin.nightly.latest.rust-analyzer
            ] ++ shots.runtimeInputs;
          }
        );
      }
    )
    // (
      let shots = final: prev: rec{
            shots = final.callPackage ./shots.nix {};
            inherit (shots) shots-unwrapped;
          };
      in {
        overlays = {
          inherit shots;
          default = shots;
        };
      }
    ));
}
