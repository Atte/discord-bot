{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, gitignore, rust-overlay, ... }: {
    nixosModules.default = import ./module.nix;
    overlays.default = final: prev: {
      discord-bot = self.packages.${prev.system}.default;
    };
  } // flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
    in
    {
      packages.default = pkgs.rustPlatform.buildRustPackage rec {
        pname = "discord-bot";
        version = "0.1.0";

        src = gitignore.lib.gitignoreSource ./.;
        cargoLock.lockFile = ./Cargo.lock;

        buildFeatures = [ ];
        buildType = "debug";

        nativeBuildInputs = [ pkgs.yarn ];

        preConfigure =
          let webui = pkgs.mkYarnPackage {
            name = "discord-bot-webui";
            src = gitignore.gitignoreSource ./webui;
            packageJSON = ./webui/package.json;
            yarnLock = ./webui/yarn.lock;
          }; in
          if builtins.any (pkgs.lib.hasPrefix "webui") buildFeatures
          then "cp -r ${webui}/libexec/discord-bot-webui/node_modules webui/"
          else "";
      };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.stable.latest.default.override {
            extensions = [ "rust-analyzer" "rust-src" ];
            targets = [ "wasm32-wasi" ];
          })
          cargo-outdated
          yarn
        ];
      };
    });
}
