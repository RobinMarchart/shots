{
  description = "screenshot gui that wraps cli tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default-linux";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
    wayfreeze = {
      url = "github:jappie3/wayfreeze";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      rust-overlay,
      flake-utils,
      wayfreeze,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        wayfreeze_pkg = wayfreeze.packages.${system}.wayfreeze;
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;
        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml
        );
        cssFilter = path: _type: builtins.match ".*css$" path != null;
        cssOrCargo = path: type: (cssFilter path type) || (craneLib.filterCargoSources path type);
        src = lib.cleanSourceWith {
          src = ./.;
          filter = cssOrCargo;
          name = "shots-src";
        };
        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          strictDeps = true;
          inherit src;
          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook
          ];
          buildInputs = with pkgs; [
            gtk4
            gsettings-desktop-schemas
            glib
          ];
        };
        runtimeDeps = [
          pkgs.slurp
          pkgs.grim
          wayfreeze_pkg
        ];

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        shots_unwrapped = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
        shots = pkgs.writeShellApplication {
          name = "shots";
          runtimeInputs = runtimeDeps;
          text = "exec \"${shots_unwrapped}/bin/shots\"";
        };
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit shots shots_unwrapped;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          shots-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "-- --deny warnings";
            }
          );

          shots-doc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });

          # Check formatting
          shots-fmt = craneLib.cargoFmt { inherit src; };

          shots-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            }
          );
        };

        packages = {
          default = shots;
          inherit shots_unwrapped;
        };

        apps.default = flake-utils.lib.mkApp { drv = shots; };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = runtimeDeps;
        };
      }
    );
}
