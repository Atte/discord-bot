use super::super::CONFIG;
use chrono::{DateTime, Utc};
use rand::{self, Rng};
use reqwest;
use serenity::utils::Colour;

#[derive(Debug, Deserialize)]
pub struct Response {
    search: Vec<Search>,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    id: usize,
    first_seen_at: Option<DateTime<Utc>>,
    file_name: Option<String>,
    image: String,
}

command!(gib(_context, message, args) {
    let args = args.full();
    let tag = CONFIG
        .gib
        .aliases
        .iter()
        .find(|(_tag, aliases)| aliases.contains(args))
        .map_or(args, |(tag, _aliases)| tag.as_ref());

    let search = if CONFIG.gib.filters.sfw.tags.is_empty() {
        tag.replace(" ", "+")
    } else {
        format!("({}) AND ({})",
            CONFIG.gib.filters.sfw.tags.join(" AND "),
            tag.replace(" ", "+"))
    };

    let link = format!("https://derpibooru.org/search.json?sf=random%3A{}&perpage=1&filter_id={}&q={}",
        rand::thread_rng().gen::<u32>(),
        CONFIG.gib.filters.sfw.filter.to_string(),
        search
    );

    let response: Response = reqwest::get(&link)?.json()?;

    if response.search.is_empty() {
        let reply = rand::thread_rng()
                        .choose(&CONFIG.gib.not_found)
                        .map_or("", |reply| reply.as_ref());

        message.reply(&reply)?;
    } else {
        let reply = rand::thread_rng()
                        .choose(&CONFIG.gib.found)
                        .map_or("", |reply| reply.as_ref());

        let first = &response.search[0];
        message.channel_id.send_message(|msg| {
            msg.embed(|mut e| {
                if let Some(ref fname) = first.file_name {
                    e = e.title(fname);
                }
                if let Some(ref timestamp) = first.first_seen_at {
                    e = e.timestamp(timestamp);
                }
                e.colour(Colour::gold())
                    .description(&reply)
                    .url(format!("https://derpibooru.org/{}", first.id))
                    .image(format!("https:{}", first.image))
            })
        })?;
    }
});
