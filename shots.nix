{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook,
  gtk4,
  gsettings-desktop-schemas,
  glib,
  writeShellApplication,
  slurp,
  grim,
  wayfreeze,
}:
let
  pname = "shots";
  version = "1.0.0";
  fileset = lib.fileset.unions [
    ./Cargo.lock
    ./Cargo.toml
    ./src
  ];
  src = lib.fileset.toSource {
    root = ./.;
    inherit fileset;
  };
  shots-unwrapped = rustPlatform.buildRustPackage {
    inherit src pname version;
    cargoLock.lockFile = ./Cargo.lock;
    nativeBuildInputs = [
      pkg-config
      wrapGAppsHook
    ];
    buildInputs = [
      gtk4
      gsettings-desktop-schemas
      glib
    ];
  };
  runtimeInputs = [
    slurp
    grim
    wayfreeze
  ];
in
writeShellApplication {
  name = "shots";
  inherit runtimeInputs;
  text = "exec \"${shots-unwrapped}/bin/shots\"";
  derivationArgs = {
    inherit pname version;
    passthru = {
      inherit shots-unwrapped runtimeInputs;
    };
  };
}
