use super::super::{
    get_data,
    limits::{EMBED_AUTHOR_LENGTH, EMBED_TITLE_LENGTH},
    DbKey, DiscordConfigKey,
};
use crate::util::{ellipsis_string, separate_thousands};
use chrono::Utc;
use futures::StreamExt;
use itertools::Itertools;
use lazy_static::lazy_static;
use mongodb::{
    bson::doc,
    options::{FindOptions, ReplaceOptions},
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Representations {
    tall: String,
}

#[command]
async fn gib(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let config = get_data::<DiscordConfigKey>(&ctx).await?;
    let db = get_data::<DbKey>(&ctx).await?;
    let collection = db.collection("gib_seen");

    let mut query = args.message().trim();
    if query.is_empty() {
        query = "*";
    }

    let response: SearchResponse = CLIENT
        .get(Url::parse_with_params(
            config.gib_endpoint.as_ref(),
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;

    let image_ids: Vec<i64> = response.images.iter().map(|image| image.id).collect();
    let seen_ids: Vec<i64> = collection
        .find(
            doc! { "image.id": { "$in": &image_ids } },
            FindOptions::builder()
                .projection(doc! { "image.id": 1 })
                .sort(doc! { "time": 1 })
                .build(),
        )
        .await?
        .filter_map(|doc| async move { doc.ok().and_then(|image| image.get_i64("image.id").ok()) })
        .collect()
        .await;
    let fresh_ids: Vec<i64> = image_ids
        .iter()
        .filter(|id| !seen_ids.contains(id))
        .copied()
        .collect();

    // order of preference: first unseen result, least recently seen result, first result
    if let Some(image) = fresh_ids
        .first()
        .or_else(|| seen_ids.first())
        .and_then(|id| response.images.iter().find(|image| &image.id == id))
        .or_else(|| response.images.first())
    {
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
        collection
            .replace_one(
                doc! { "image.id": image.id },
                doc! { "image": image, "time": Utc::now() },
                ReplaceOptions::builder().upsert(true).build(),
            )
            .await?;
    } else {
        msg.reply(&ctx, "No results").await?;
    }

    Ok(())
}
