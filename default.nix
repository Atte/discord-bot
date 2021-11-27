{ features ? [ ]
, pkgs ? import (fetchTarball "channel:nixpkgs-unstable") { }
}:

let lib = pkgs.lib; in

with import (fetchTarball "https://github.com/hercules-ci/gitignore.nix/archive/master.tar.gz") { inherit lib; };

pkgs.rustPlatform.buildRustPackage {
  pname = "discord-bot";
  version = "0.1.0";

  src = gitignoreSource ./.;
  cargoLock.lockFile = ./Cargo.lock;
  buildFeatures = features;

  nativeBuildInputs = [ pkgs.yarn ];
}
