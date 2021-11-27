{ features ? [ ]
, pkgs ? import (fetchTarball "channel:nixpkgs-unstable") { }
, lib ? pkgs.lib
}:

with import (fetchTarball "https://github.com/hercules-ci/gitignore.nix/tarball/5b9e0ff9d3b551234b4f3eb3983744fa354b17f1") { inherit lib; };

pkgs.rustPlatform.buildRustPackage {
  pname = "discord-bot";
  version = "0.1.0";

  src = gitignoreSource ./.;
  cargoLock.lockFile = ./Cargo.lock;

  buildFeatures = features;
  buildType = "debug";

  nativeBuildInputs = [ pkgs.yarn ];
}
