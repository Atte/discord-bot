{ lib, config, ... }:

let
  cfg = config.services.discord-bot;
in
{
  options.services.discord-bot = {
    enable = lib.mkEnableOption "discord-bot";

    package = lib.mkOption {
      type = lib.types.package;
      default = import ./default.nix { features = cfg.features; };
    };

    features = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "discord-bot";
    };

    port = lib.mkOption {
      type = lib.types.int;
      default = 56456;
    };

    config = lib.mkOption {
      type = lib.types.path;
    };

    secret-key = lib.mkOption {
      type = lib.types.str;
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    users.users."${cfg.user}".isSystemUser = lib.mkDefault true;

    systemd.services.discord-bot = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      serviceConfig = {
        User = cfg.user;
        ExecStart = "${cfg.package}/bin/discord-bot";
        Restart = "on-failure";
      };
      environment = {
        CONFIG_PATH = cfg.config;

        ROCKET_SECRET_KEY = cfg.secret-key;
        ROCKET_PORT = toString cfg.port;

        RUST_LOG = "warn,discord_bot=trace";
        RUST_BACKTRACE = "1";
      };
    };
  };
}