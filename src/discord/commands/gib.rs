use super::super::{
    limits::{EMBED_AUTHOR_LENGTH, EMBED_TITLE_LENGTH},
    DiscordConfigKey,
};
use crate::util::{ellipsis_string, separate_thousands};
use itertools::Itertools;
use lazy_static::lazy_static;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_default_from_null;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use std::time::Duration;

lazy_static! {
    static ref CLIENT: Client = Client::builder()
        .user_agent("discord-bot (by Atte)")
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Unable to create reqwest Client!");
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    images: Vec<Image>,
    total: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct Image {
    id: i64,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    name: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    tags: Vec<String>,
    source_url: Option<String>,
    first_seen_at: Option<String>,
    representations: Representations,
}

#[derive(Debug, Clone, Deserialize)]
struct Representations {
    tall: String,
}

#[command]
async fn gib(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut query = args.message().trim();
    if query.is_empty() {
        query = "*";
    }
    let response: SearchResponse = CLIENT
        .get(Url::parse_with_params(
            DiscordConfigKey::get(&ctx).await.gib_endpoint.as_ref(),
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;
    if let Some(image) = response.images.first() {
        let artists = image
            .tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("artist:"))
            .join(", ");
        msg.channel_id
            .send_message(&ctx, |message| {
                message.embed(|embed| {
                    if !artists.is_empty() {
                        embed.author(|author| {
                            author.name(ellipsis_string(artists, EMBED_AUTHOR_LENGTH))
                        });
                    }
                    if let Some(ref timestamp) = image.first_seen_at {
                        embed.timestamp::<&str>(timestamp);
                    }
                    embed
                        .title(if image.name.is_empty() {
                            String::from("(no title)")
                        } else {
                            ellipsis_string(&image.name, EMBED_TITLE_LENGTH)
                        })
                        .url(
                            image
                                .source_url
                                .clone()
                                .unwrap_or_else(|| format!("https://derpibooru.org/{}", image.id)),
                        )
                        .image(&image.representations.tall)
                        .footer(|footer| {
                            footer.text(format!(
                                "{} results",
                                separate_thousands(response.total.to_string())
                            ))
                        })
                })
            })
            .await?;
    } else {
        msg.reply(&ctx, "No results").await?;
    }

    Ok(())
}
