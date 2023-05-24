{
  inputs = {
    nixpkgs.url = "nixpkgs";

    flake-utils.url = "flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
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
      nodejs = pkgs.nodejs-16_x;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = pkgs.rust-bin.stable.latest.minimal;
        rustc = pkgs.rust-bin.stable.latest.minimal;
      };
    in
    {
      packages.default = pkgs.lib.makeOverridable
        ({ features }: rustPlatform.buildRustPackage {
          pname = "discord-bot";
          version = "0.1.1";

          src = gitignore.lib.gitignoreSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildFeatures = features;
          buildType = "debug";

          nativeBuildInputs = with pkgs; [
            (yarn.override { inherit nodejs; })
          ];

          preConfigure =
            let webui = pkgs.mkYarnPackage {
              name = "discord-bot-webui";
              src = gitignore.lib.gitignoreSource ./webui;
              packageJSON = ./webui/package.json;
              yarnLock = ./webui/yarn.lock;
            }; in
            if builtins.any (pkgs.lib.hasPrefix "webui") features
            then "cp -r ${webui}/libexec/discord-bot-webui/node_modules webui/"
            else "";
        })
        { features = [ ]; };

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.stable.latest.default.override {
            extensions = [ "rust-analyzer" "rust-src" ];
            targets = [ "aarch64-unknown-linux-gnu" ];
          })
          cargo-outdated
          nodejs
          (yarn.override { inherit nodejs; })
        ];
      };
    });
}
