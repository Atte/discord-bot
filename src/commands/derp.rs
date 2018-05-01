use super::super::CONFIG;
use rand::{self, Rng};
use reqwest;
use reqwest::Url;
use serenity::utils::Colour;

#[derive(Deserialize)]
pub struct DerpResponse {
    pub search: Vec<DerpSearch>,
}

#[derive(Deserialize)]
pub struct DerpSearch {
    pub id: u64,
    pub image: String,
    pub representations: DerpSearchImages,
}

#[derive(Deserialize)]
pub struct DerpSearchImages {
    pub thumb: String,
    pub medium: String,
}

command!(gib(_context, message, args) {
    let filter = CONFIG.gib.filters.sfw.filter.to_string();
    let mut input = args.full();

    'outer: for ( tag, aliases ) in CONFIG.gib.aliases.iter() {
        for alias in aliases {
            if input == alias {
                input = tag;
                break 'outer;
            }
        }
    }

    let mut search;

    if CONFIG.gib.filters.sfw.tags.len() > 0 {
        search = format!("({}) AND ({})",
            CONFIG.gib.filters.sfw.tags.join(" AND "),
            input.replace(" ", "+"));
    }else{
        search = format!("{}",
            input.replace(" ", "+"));
    }

    let link = format!("https://derpibooru.org/search.json?min_score=100&sf=random%3A{}&perpage=1&filter_id={}&q={}",
        rand::thread_rng().gen::<u32>(),
        filter,
        search
    );

    let mut curl = Url::parse(&link)?;
    let mut res = reqwest::get(curl)?;
    let json: DerpResponse = res.json()?;

    if json.search.len() == 0 {
        let reply = rand::thread_rng()
                        .choose(&CONFIG.gib.not_found)
                        .map_or("", |reply| reply.as_ref());

        message.reply( &reply )?;
    }else{
        let reply = rand::thread_rng()
                        .choose(&CONFIG.gib.found)
                        .map_or("", |reply| reply.as_ref());

        let first = &json.search[0];
        message.channel_id.send_message(|msg| {
            msg.embed(|e|
                e.colour(Colour::gold())
                .description( &reply )
                .field("Link", format!("https://derpibooru.org/{}",first.id), false)
                .image(format!("http:{}",first.representations.medium))
            )
        })?;
    }
});
