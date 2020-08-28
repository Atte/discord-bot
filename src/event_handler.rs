use serenity::async_trait;
use serenity::client::EventHandler;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {}
