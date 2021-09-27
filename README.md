discord-bot
===========

[![build](https://github.com/Atte/discord-bot/workflows/build/badge.svg)](https://github.com/Atte/discord-bot/actions)
[![codecov](https://codecov.io/gh/Atte/discord-bot/branch/master/graph/badge.svg?token=YWH961SA18)](https://codecov.io/gh/Atte/discord-bot)

Cargo features
--------------

* webui
* berrytube
* cron

Configuration
-------------

By default configuration is read from `config.toml` on startup. To change the filename use the `CONFIG_PATH` environment variable. Most string values in the configuration support environment variable substitution.

Inviting the bot
----------------

https://discord.com/api/oauth2/authorize?client_id=349895102115741697&scope=bot&permissions=2416299200

Replace the client_id with your own bot account's ID.
