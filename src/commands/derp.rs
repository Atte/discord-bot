use super::super::CONFIG;
use chrono::{DateTime, Utc};
use rand::{self, Rng};
use regex::Regex;
use reqwest;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Colour;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Response {
    search: Vec<SearchResponse>,
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
    medium: String,
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

    let search: Vec<_> = args.full()
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
        .collect();

    let url = Url::parse_with_params(
        "https://derpibooru.org/search.json",
        &[
            ("sf", format!("random:{}", rand::thread_rng().gen::<u32>())),
            ("perpage", "1".to_owned()),
            ("filter_id", CONFIG.gib.filter.to_string()),
            ("q", search.join(",")),
        ],
    )?;

    let response: Response = reqwest::get(&url.as_ref().replace("%2B", "+"))?.json()?;

    if response.search.is_empty() {
        message.reply(
            rand::thread_rng()
                .choose(&CONFIG.gib.not_found)
                .map_or("", |reply| reply.as_ref()),
        )?;
    } else if let Some(first) = response.search.first() {
        let url = Url::parse("https://derpibooru.org/")?.join(&first.id.to_string())?;
        let image = Url::parse("https://derpicdn.net/")?.join(
            first
                .representations
                .as_ref()
                .map_or(&first.image, |reprs| &reprs.medium),
        )?;
        let artists: Vec<_> = first
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
        let description = first.description.as_ref().map(|desc| {
            let d = REGEXES.iter().fold(desc.to_owned(), |acc, x| {
                x.0.replace_all(&acc, x.1).into_owned()
            });
            if d.len() > CONFIG.discord.long_msg_threshold {
                format!("{}\u{2026}", &d[..CONFIG.discord.long_msg_threshold])
            } else {
                d.clone()
            }
        });

        message.channel_id.send_message(|msg| {
            msg.embed(|mut e| {
                if let Some(ref timestamp) = first.first_seen_at {
                    e = e.timestamp(timestamp);
                }
                if !artists.is_empty() {
                    e = e.author(|a| a.name(&artists.join(" & ")));
                }
                if let Some(ref desc) = description {
                    e = e.description(desc);
                }
                e.colour(Colour::gold())
                    .title(if let Some(ref fname) = first.file_name {
                        fname
                    } else {
                        "<no filename>"
                    })
                    .url(url)
                    .image(image)
            })
        })?;
    }
    Ok(())
}
