use super::super::{CACHE, CONFIG};
use chrono::{DateTime, Utc};
use digit_group::FormatGroup;
use rand::{self, Rng};
use regex::Regex;
use reqwest;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::{Colour, MessageBuilder};
use url::Url;

const MAX_ARTISTS: usize = 4;

#[derive(Debug, Deserialize)]
pub struct Response {
    search: Vec<SearchResponse>,
    total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    id: usize,
    image: String,
    tags: String,
    description: Option<String>,
    file_name: Option<String>,
    first_seen_at: Option<DateTime<Utc>>,
    representations: Option<RepresentationList>,
}

#[derive(Debug, Deserialize)]
pub struct RepresentationList {
    tall: String,
}

pub fn gib(_: &mut Context, message: &Message, args: Args) -> Result<(), CommandError> {
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
        ].into_iter()
            .map(|x| (Regex::new(x.0).unwrap(), x.1))
            .collect();
    }

    let search = args
        .full()
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
        }).collect::<Vec<_>>()
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

    let response: Response = reqwest::get(&url.as_ref().replace("%2B", "+"))?.json()?;

    if response.search.is_empty() {
        message.reply(
            rand::thread_rng()
                .choose(&CONFIG.gib.not_found)
                .map_or("", |reply| reply.as_ref()),
        )?;
    } else if let Some(result) = CACHE.with(|cache| {
        let result = response
            .search
            .iter()
            .find(|result| !cache.gib_seen.contains(&result.id))
            .or_else(|| response.search.first());
        if let Some(result) = result {
            cache.gib_seen.insert(0, result.id);
            cache.gib_seen.truncate(CONFIG.gib.history);
        }
        result
    })? {
        let url = Url::parse("https://derpibooru.org/")?.join(&result.id.to_string())?;
        let image = Url::parse("https://derpicdn.net/")?.join(
            result
                .representations
                .as_ref()
                .map_or(&result.image, |reprs| &reprs.tall),
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
            }).collect();
        let description = result.description.as_ref().map(|desc| {
            let desc = REGEXES
                .iter()
                .fold(desc.to_owned(), |acc, (pattern, replacement)| {
                    pattern.replace_all(&acc, *replacement).into_owned()
                });
            if desc.len() > CONFIG.discord.long_msg_threshold {
                format!("{}\u{2026}", &desc[..CONFIG.discord.long_msg_threshold])
            } else {
                desc.clone()
            }
        });

        message.channel_id.send_message(|msg| {
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
                if let Some(ref desc) = description {
                    e = e.description(desc);
                }
                e.colour(Colour::gold())
                    .title(if let Some(ref fname) = result.file_name {
                        MessageBuilder::new().push_safe(fname).build()
                    } else {
                        "<no filename>".to_owned()
                    }).url(url)
                    .image(image)
                    .footer(|f| {
                        f.text(&format!("Out of {} results", response.total.format_si('.')))
                    })
            })
        })?;
    }
    Ok(())
}
