use crate::{db, CONFIG};
use chrono::{DateTime, Utc};
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

#[derive(Debug, Deserialize)]
pub struct Response {
    search: Vec<SearchResponse>,
    total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    id: u32,
    image: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    tags: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    description: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    file_name: String,
    first_seen_at: Option<DateTime<Utc>>,
    representations: Option<RepresentationList>,
}

#[derive(Debug, Deserialize)]
pub struct RepresentationList {
    tall: Option<String>,
}

#[command]
#[description("Gibs pics from derpibooru.")]
#[usage("[tags\u{2026}]")]
#[bucket("derp")]
pub fn gib(context: &mut Context, message: &Message, args: Args) -> CommandResult {
    lazy_static! {
        static ref REGEXES: Vec<(Regex, &'static str)> = [
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
        "https://derpibooru.org/search.json",
        &[
            ("sf", "random".to_owned()),
            ("perpage", "50".to_owned()),
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

    let response: Response = reqwest::blocking::get(&url.as_ref().replace("%2B", "+"))?.json()?;

    if response.search.is_empty() {
        message.reply(
            &context,
            CONFIG
                .gib
                .not_found
                .choose(&mut rand::thread_rng())
                .map_or("", |reply| reply.as_ref()),
        )?;
    } else if let Some(result) = db::with_db(|conn| {
        let unseen = response
            .search
            .iter()
            .find(|result| !db::gib_is_seen(&conn, result.id).unwrap_or(false))
            .or_else(|| response.search.first());
        if let Some(unseen) = unseen {
            db::gib_seen(&conn, unseen.id)?;
        }
        Ok(unseen)
    })? {
        let url = Url::parse("https://derpibooru.org/")?.join(&result.id.to_string())?;
        let image = Url::parse("https://derpicdn.net/")?.join(
            result
                .representations
                .as_ref()
                .and_then(|reprs| reprs.tall.as_ref())
                .unwrap_or(&result.image),
        )?;
        let artists: Vec<_> = result
            .tags
            .split(", ")
            .filter_map(|tag| {
                if tag.starts_with("artist:") {
                    Some(&tag[7..])
                } else {
                    None
                }
            })
            .collect();

        let full_desc = REGEXES.iter().fold(
            result.description.to_owned(),
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
                if let Some(ref timestamp) = result.first_seen_at {
                    e = e.timestamp(timestamp);
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
                    .title(if result.file_name.is_empty() {
                        "<no filename>".to_owned()
                    } else {
                        MessageBuilder::new().push_safe(&result.file_name).build()
                    })
                    .url(url)
                    .image(image)
                    .footer(|f| {
                        f.text(&format!("Out of {} results", response.total.format_si('.')))
                    })
            })
        })?;
    }
    Ok(())
}
