{ features ? [ ]
, pkgs ? import <nixpkgs> { }
, lib ? pkgs.lib
, sources ? import ./nix/sources.nix
}:

let
  gitignore = import sources."gitignore.nix" { inherit lib; };
in
pkgs.rustPlatform.buildRustPackage {
  pname = "discord-bot";
  version = "0.1.0";

  src = gitignore.gitignoreSource ./.;
  cargoLock.lockFile = ./Cargo.lock;

  buildFeatures = features;
  buildType = "debug";

  nativeBuildInputs = [ pkgs.yarn ];

  preConfigure =
    let webui = pkgs.mkYarnPackage {
      name = "discord-bot-webui";
      src = gitignore.gitignoreSource ./webui;
      packageJSON = ./webui/package.json;
      yarnLock = ./webui/yarn.lock;
    }; in
    if builtins.any (lib.hasPrefix "webui") features
    then "cp -r ${webui}/libexec/discord-bot-webui/node_modules webui/"
    else "";
}
