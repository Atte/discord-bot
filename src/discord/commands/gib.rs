use super::super::{
    get_data, get_data_or_insert_with, limits::EMBED_FIELD_VALUE_LENGTH, ConfigKey, DbKey,
};
use crate::util::{ellipsis_string, separate_thousands_unsigned};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use itertools::Itertools;
use mongodb::bson::{doc, to_bson, Document};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};
use serenity::{
    all::{CreateEmbed, CreateEmbedFooter, CreateMessage},
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

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    id: i64,
    #[serde_as(as = "DefaultOnNull")]
    pub tags: Vec<String>,
    source_url: Option<String>,
    first_seen_at: Option<String>,
    representations: Representations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Representations {
    tall: String,
}

#[derive(Debug)]
struct ClientKey;

impl TypeMapKey for ClientKey {
    type Value = Result<Client, String>;
}

const COLLECTION_NAME: &str = "gib-seen";

pub async fn derpibooru_search(
    ctx: &Context,
    query: &str,
) -> CommandResult<Option<(Image, usize)>> {
    let config = get_data::<ConfigKey>(ctx).await?;
    let collection = get_data::<DbKey>(ctx)
        .await?
        .collection::<Document>(COLLECTION_NAME);
    let client = get_data_or_insert_with::<ClientKey, _>(ctx, || {
        Client::builder()
            .user_agent(config.gib.user_agent.to_string())
            .connect_timeout(Duration::from_secs(10))
            .build()
            // simplify error to a `String` to make it `impl Clone`
            .map_err(|err| format!("Unable to create reqwest::Client: {err:?}"))
    })
    .await?;

    let response: SearchResponse = client
        .get(Url::parse_with_params(
            config.gib.endpoint.as_ref(),
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;

    let images: Vec<&Image> = response
        .images
        .iter()
        /*
        // drop images where all artist are shy
        .filter(|image| {
            let mut artists = image
                .tags
                .iter()
                .filter_map(|tag| tag.strip_prefix("artist:"));
            artists.by_ref().count() == 0
                || artists.any(|artist| !config.gib.shy_artists.contains(artist))
        })
        */
        .collect();

    let image_ids: Vec<i64> = images.iter().map(|image| image.id).collect();
    let seen_ids: Vec<i64> = collection
        .find(doc! { "image.id": { "$in": image_ids.as_slice() } })
        .projection(doc! { "image.id": 1 })
        .sort(doc! { "time": 1 })
        .await?
        .filter_map(|doc| async move {
            doc.ok().and_then(|doc| {
                doc.get_document("image")
                    .and_then(|image| image.get_i64("id"))
                    .ok()
            })
        })
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
        .copied()
        .cloned()
    {
        collection
            .update_one(
                doc! { "image.id": image.id },
                doc! { "$set": { "image": to_bson(&image)?, "time": Utc::now() } },
            )
            .upsert(true)
            .await?;
        Ok(Some((image, response.total)))
    } else {
        Ok(None)
    }
}

pub async fn derpibooru_embed(
    ctx: &Context,
    msg: &Message,
    image: &Image,
    total: usize,
) -> CommandResult {
    let artists = image
        .tags
        .iter()
        .filter_map(|tag| tag.strip_prefix("artist:"))
        .join(", ");
    msg.channel_id
        .send_message(
            &ctx,
            CreateMessage::new().embed({
                let mut embed = CreateEmbed::new().field(
                    "Post",
                    format!("https://derpibooru.org/{}", image.id),
                    true,
                );
                if !artists.is_empty() {
                    embed = embed.field(
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
                    embed = embed.timestamp(DateTime::parse_from_rfc3339(timestamp)?)
                }
                embed
                    .image(&image.representations.tall)
                    .footer(CreateEmbedFooter::new(format!(
                        "{} results",
                        separate_thousands_unsigned(total)
                    )))
            }),
        )
        .await?;
    Ok(())
}

#[command]
#[aliases(give, derpi, derpibooru)]
#[description("Gib pics from Derpibooru")]
#[usage("[tags\u{2026}]")]
async fn gib(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    log::debug!("in gib {msg:?} {args:?}");

    let mut query = args.message().trim();
    if query.is_empty() {
        query = "*";
    }

    if let Some((image, total)) = derpibooru_search(ctx, query).await? {
        derpibooru_embed(ctx, msg, &image, total).await?;
    } else {
        msg.reply(&ctx, "No results").await?;
    }

    Ok(())
}
