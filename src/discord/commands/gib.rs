use super::super::{
    get_data, get_data_or_insert_with, limits::EMBED_FIELD_VALUE_LENGTH, DbKey, DiscordConfigKey,
};
use crate::util::{ellipsis_string, separate_thousands_unsigned};
use chrono::Utc;
use futures::StreamExt;
use itertools::Itertools;
use mongodb::{
    bson::{doc, to_bson},
    options::FindOptions,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_default_from_null;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::TypeMapKey,
};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    images: Vec<Image>,
    total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Image {
    id: i64,
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

struct ClientKey;

impl TypeMapKey for ClientKey {
    type Value = Result<Client, String>;
}

const COLLECTION_NAME: &str = "gib-seen";

#[command]
#[description("Gib pics from Derpibooru")]
#[usage("[tags\u{2026}]")]
async fn gib(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let config = get_data::<DiscordConfigKey>(&ctx).await?;
    let collection = get_data::<DbKey>(&ctx).await?.collection(COLLECTION_NAME);
    let client = get_data_or_insert_with::<ClientKey, _>(&ctx, || {
        Client::builder()
            .user_agent(config.gib.user_agent.to_string())
            .connect_timeout(Duration::from_secs(10))
            .build()
            // simplify error to a `String` to make it `impl Clone`
            .map_err(|err| format!("Unable to create reqwest::Client: {}", err))
    })
    .await?;

    let mut query = args.message().trim();
    if query.is_empty() {
        query = "*";
    }

    let response: SearchResponse = client
        .get(Url::parse_with_params(
            config.gib.endpoint.as_ref(),
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;

    // drop images where the only artist is a shy one
    let images: Vec<&Image> = response
        .images
        .iter()
        .filter(|image| {
            let mut artists = image
                .tags
                .iter()
                .filter_map(|tag| tag.strip_prefix("artist:"));
            artists.by_ref().count() == 1
                && config
                    .gib
                    .shy_artists
                    // unwrap is safe: length is checked above
                    .contains(artists.next().unwrap())
        })
        .collect();

    let image_ids: Vec<i64> = images.iter().map(|image| image.id).collect();
    let seen_ids: Vec<i64> = collection
        .find(
            doc! { "image.id": { "$in": &image_ids } },
            FindOptions::builder()
                .projection(doc! { "image.id": 1 })
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
        .and_then(|id| images.iter().find(|image| &image.id == id))
        .or_else(|| images.first())
    {
        let artists = image
            .tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("artist:"))
            .join(", ");
        msg.channel_id
            .send_message(&ctx, |message| {
                message.embed(|embed| {
                    embed.field("Post", format!("https://derpibooru.org/{}", image.id), true);
                    if !artists.is_empty() {
                        embed.field(
                            if artists.contains(", ") {
                                "Artists"
                            } else {
                                "Artist"
                            },
                            ellipsis_string(artists, EMBED_FIELD_VALUE_LENGTH),
                            true,
                        );
                    }
                    /*
                    if let Some(ref source_url) = image.source_url {
                        if source_url.len() <= EMBED_FIELD_VALUE_LENGTH {
                            embed.field("Source", source_url, false);
                        }
                    }
                    */
                    if let Some(ref timestamp) = image.first_seen_at {
                        embed.timestamp::<&str>(timestamp);
                    }
                    embed.image(&image.representations.tall).footer(|footer| {
                        footer.text(format!(
                            "{} results",
                            separate_thousands_unsigned(response.total)
                        ))
                    })
                })
            })
            .await?;
        collection
            .insert_one(doc! { "image": to_bson(image)?, "time": Utc::now() }, None)
            .await?;
    } else {
        msg.reply(&ctx, "No results").await?;
    }

    Ok(())
}
