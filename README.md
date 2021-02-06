discord-bot
===========

![build](https://github.com/Atte/discord-bot/workflows/build/badge.svg)
[![codecov](https://codecov.io/gh/Atte/discord-bot/branch/master/graph/badge.svg?token=YWH961SA18)](https://codecov.io/gh/Atte/discord-bot)

Building on nightly
-------------------

The default configuration may fail to build on a nightly Rust toolchain. To build on nightly, pass `--no-default-features --features nightly` to your Cargo commands.

Configuration
-------------

By default configuration is read from `config.toml` on startup. To change the filename use the `CONFIG_PATH` environment variable. Most string values in the configuration support environment variable substitution.
