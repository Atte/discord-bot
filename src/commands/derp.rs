use crate::{db, CONFIG};
use digit_group::FormatGroup;
use lazy_static::lazy_static;
use log::trace;
use rand::{self, seq::SliceRandom};
use regex::Regex;
use reqwest;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_default_from_null;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::{Colour, MessageBuilder},
};
use url::Url;

const MAX_ARTISTS: usize = 4;

lazy_static! {
    static ref FORMATTING_REGEXES: Vec<(Regex, &'static str)> = [
        // bold
        (
            r"(?P<s1>^|\s)\*(?P<t>[\w ]+?)\*(?P<s2>\s|$)",
            "$s1**$t**$s2"
        ),
        // italics
        (r"(?P<s1>^|\s)_(?P<t>[\w ]+?)_(?P<s2>\s|$)", "$s1*$t*$s2"),
        // underline
        (
            r"(?P<s1>^|\s)\+(?P<t>[\w ]+?)\+(?P<s2>\s|$)",
            r"$s1__${t}__$s2"
        ),
        // inline code
        (r"(?P<s1>^|\s)@(?P<t>[\w ]+?)@(?P<s2>\s|$)", "$s1`$t`$s2"),
        // strikethrough
        (
            r"(?P<s1>^|\s)\-(?P<t>[\w ]+?)\-(?P<s2>\s|$)",
            "$s1~~$t~~$s2"
        ),
        // superscript
        (r"(?P<s1>^|\s)\^(?P<t>[\w ]+?)\^(?P<s2>\s|$)", "$s1$t$s2"),
        // subscript
        (r"(?P<s1>^|\s)\~(?P<t>[\w ]+?)\~(?P<s2>\s|$)", "$s1$t$s2"),
        // block quote
        (r"\[bq\]", ""),
        (r"\[/bq\]", ""),
        // spoiler
        (r"\[spoiler\]", ""),
        (r"\[/spoiler\]", ""),
        // link
        (r#""(?P<t>.+?)":(?P<u>\S+)"#, "[$t]($u)"),
        // image embed
        (r"(?P<s1>^|\s)!(?P<t>\S+?)!(?P<s2>\s)", "$s1[Image]($t)$s2"),
        // no parse
        (r"\[==(?P<t>[\w ]+?)==\]", "$t"),
    ].iter()
        .map(|x| (Regex::new(x.0).unwrap(), x.1))
        .collect();
}

#[derive(Debug, Deserialize)]
pub struct ImageResponse {
    images: Vec<Image>,
    total: usize,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    id: u32,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    tags: Vec<String>,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    description: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    name: String,
    first_seen_at: Option<String>,
    representations: RepresentationList,
}

#[derive(Debug, Deserialize)]
pub struct RepresentationList {
    tall: String,
}

#[command]
#[description("Gib pics from Derpibooru")]
#[usage("[tags\u{2026}]")]
#[bucket("derp")]
pub fn gib(context: &mut Context, message: &Message, args: Args) -> CommandResult {
    let search = args
        .message()
        .split(',')
        .map(|arg| {
            let arg = arg.trim();
            CONFIG
                .gib
                .aliases
                .iter()
                .find(|(_tag, aliases)| aliases.contains(arg))
                .map_or(arg, |(tag, _aliases)| tag.as_ref())
                .replace(" ", "+")
        })
        .collect::<Vec<_>>()
        .join(",");

    let url = Url::parse_with_params(
        "https://derpibooru.org/api/v1/json/search/images",
        &[
            ("sf", "random".to_owned()),
            ("per_page", "50".to_owned()),
            ("filter_id", CONFIG.gib.filter.to_string()),
            (
                "q",
                if search.is_empty() {
                    "*".to_owned()
                } else {
                    search
                },
            ),
        ],
    )?;
    trace!("Search URL: {}", url);

    match reqwest::blocking::get(&url.as_ref().replace("%2B", "+")) {
        Err(_) => {
            message.reply(&context, "Can't connect to Derpi <:thisisfine:364466172714024980>")?;
        }
        Ok(response) => match response.json::<ImageResponse>() {
            Err(_) => {
                message.reply(&context, "Derpi responded with garbage <:rdwut:310577997633814549>")?;
            }
            Ok(response) => {
                if response.images.is_empty() {
                    message.reply(
                        &context,
                        CONFIG
                            .gib
                            .not_found
                            .choose(&mut rand::thread_rng())
                            .map_or("", |reply| reply.as_ref()),
                    )?;
                } else if let Some(image) = find_unseen(&response.images)? {
                    send_image(&context, &message, &image, response.total)?;
                }
            }
        },
    }

    Ok(())
}

fn find_unseen(images: &[Image]) -> db::Result<Option<&Image>> {
    db::with_db(|conn| {
        let unseen = images
            .iter()
            .find(|result| !db::gib_is_seen(&conn, result.id).unwrap_or(false))
            .or_else(|| images.first());
        if let Some(unseen) = unseen {
            db::gib_seen(&conn, unseen.id)?;
        }
        Ok(unseen)
    })
}

fn send_image(context: &Context, message: &Message, image: &Image, count: usize) -> CommandResult {
    let url = Url::parse("https://derpibooru.org/")?.join(&image.id.to_string())?;
    let artists: Vec<_> = image
        .tags
        .iter()
        .filter_map(|tag| {
            if tag.starts_with("artist:") {
                Some(&tag[7..])
            } else {
                None
            }
        })
        .collect();

    let full_desc = FORMATTING_REGEXES.iter().fold(
        image.description.to_owned(),
        |acc, (pattern, replacement)| pattern.replace_all(&acc, *replacement).into_owned(),
    );
    let description = if full_desc.len() > CONFIG.discord.long_msg_threshold {
        format!(
            "{}\u{2026}",
            &full_desc[..CONFIG.discord.long_msg_threshold]
        )
    } else {
        full_desc
    };

    message.channel_id.send_message(&context, |msg| {
        msg.embed(|mut e| {
            if let Some(ref timestamp) = image.first_seen_at {
                e = e.timestamp(timestamp.to_owned());
            }
            if !artists.is_empty() {
                if artists.len() > MAX_ARTISTS {
                    e = e.author(|a| {
                        a.name(&format!(
                            "{} & {} others",
                            artists[..MAX_ARTISTS - 1].join(" & "),
                            artists.len() - (MAX_ARTISTS - 1)
                        ))
                    });
                } else {
                    e = e.author(|a| a.name(&artists.join(" & ")));
                }
            }
            if !description.is_empty() {
                e = e.description(description);
            }
            e.colour(Colour::GOLD)
                .title(if image.name.is_empty() {
                    "<no filename>".to_owned()
                } else {
                    MessageBuilder::new().push_safe(&image.name).build()
                })
                .url(url)
                .image(&image.representations.tall)
                .footer(|f| f.text(&format!("Out of {} results", count.format_si('.'))))
        })
    })?;

    Ok(())
}
