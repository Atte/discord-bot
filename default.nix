{ features ? [ ]
, pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
}:

with import
  (fetchTarball {
    url = "https://github.com/hercules-ci/gitignore.nix/tarball/5b9e0ff9d3b551234b4f3eb3983744fa354b17f1";
    sha256 = "01l4phiqgw9xgaxr6jr456qmww6kzghqrnbc7aiiww3h6db5vw53";
  })
{ inherit lib; };

pkgs.rustPlatform.buildRustPackage {
  pname = "discord-bot";
  version = "0.1.0";

  src = gitignoreSource ./.;
  cargoLock.lockFile = ./Cargo.lock;

  buildFeatures = features;
  buildType = "debug";

  nativeBuildInputs = [ pkgs.yarn ];

  preConfigure =
    let webui = pkgs.mkYarnPackage {
      name = "discord-bot-webui";
      src = gitignoreSource ./webui;
      packageJSON = ./webui/package.json;
      yarnLock = ./webui/yarn.lock;
    }; in
    if builtins.any (lib.hasPrefix "webui") features
    then "cp -r ${webui}/libexec/discord-bot-webui/node_modules webui/"
    else "";
}
