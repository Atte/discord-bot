use super::super::{
    get_data, get_data_or_insert_with, limits::EMBED_FIELD_VALUE_LENGTH, ConfigKey, DbKey,
};
use crate::{
    discord::Context,
    util::{ellipsis_string, separate_thousands_unsigned},
    Result,
};
use chrono::{DateTime, Utc};
use color_eyre::eyre::eyre;
use futures::StreamExt;
use itertools::Itertools;
use mongodb::bson::{doc, to_bson, Document};
use poise::{command, CreateReply};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};
use serenity::{
    all::{CreateEmbed, CreateEmbedFooter},
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

pub async fn derpibooru_search(ctx: &Context<'_>, query: &str) -> Result<Option<(Image, usize)>> {
    let config = get_data::<ConfigKey>(ctx.serenity_context()).await?;
    let collection = get_data::<DbKey>(ctx.serenity_context())
        .await?
        .collection::<Document>(COLLECTION_NAME);
    let client = get_data_or_insert_with::<ClientKey, _>(ctx.serenity_context(), || {
        Client::builder()
            .user_agent(config.gib.user_agent.to_string())
            .connect_timeout(Duration::from_secs(10))
            .build()
            // simplify error to a `String` to make it `Clone`
            .map_err(|err| format!("Unable to create reqwest::Client: {err:?}"))
    })
    .await
    .map_err(|err| eyre!(err))?;

    let response: SearchResponse = client
        .get(Url::parse_with_params(
            config.gib.endpoint.as_ref(),
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;

    let image_ids: Vec<i64> = response.images.iter().map(|image| image.id).collect();
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
        .and_then(|id| response.images.iter().find(|image| &image.id == id))
        .or_else(|| response.images.first())
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

pub async fn derpibooru_embed(ctx: &Context<'_>, image: &Image, total: usize) -> Result<()> {
    let artists = image
        .tags
        .iter()
        .filter_map(|tag| tag.strip_prefix("artist:"))
        .join(", ");
    ctx.send(CreateReply::default().embed({
        let mut embed =
            CreateEmbed::new().field("Post", format!("https://derpibooru.org/{}", image.id), true);
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
        if let Some(ref timestamp) = image.first_seen_at {
            embed = embed.timestamp(DateTime::parse_from_rfc3339(timestamp)?);
        }
        embed
            .image(&image.representations.tall)
            .footer(CreateEmbedFooter::new(format!(
                "{} results",
                separate_thousands_unsigned(total)
            )))
    }))
    .await?;
    Ok(())
}

/// Gib pics matching the given tags from Derpibooru
#[command(
    prefix_command,
    category = "Horse",
    aliases("give", "derpi", "derpibooru"),
    invoke_on_edit,
    track_deletion
)]
pub async fn gib(ctx: Context<'_>, #[rest] query: Option<String>) -> Result<()> {
    let query = query.as_ref().map_or("*", |s| s.trim());

    if let Some((image, total)) = derpibooru_search(&ctx, query).await? {
        derpibooru_embed(&ctx, &image, total).await?;
    } else {
        ctx.reply("No results").await?;
    }

    Ok(())
}
