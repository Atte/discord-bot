use super::super::util::use_emoji;

command!(ping(_context, message) {
    message.reply(&format!("Pong! {}", use_emoji("DIDNEYWORL")))?;
});
