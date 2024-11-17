{ pkgs, lib, config, ... }:

let
  cfg = config.services.discord-bot;
in
{
  options.services.discord-bot = {
    enable = lib.mkEnableOption "discord-bot";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.discord-bot.override {
        features = cfg.features;
      };
    };

    features = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "discord-bot";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "discord-bot";
    };

    config = lib.mkOption {
      type = lib.types.path;
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    users.users."${cfg.user}" = {
      isSystemUser = lib.mkDefault true;
      group = cfg.group;
    };
    users.groups."${cfg.group}" = { };

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

        RUST_LOG = "warn,discord_bot=trace";
        RUST_BACKTRACE = "1";
      };
    };
  };
}
