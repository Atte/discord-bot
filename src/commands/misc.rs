use super::super::util::use_emoji;
use super::super::CONFIG;
use meval;
use rand::{self, Rng};
use regex::{Captures, Regex};
use reqwest;
use reqwest::Url;
use serenity::utils::Colour;
use serenity::CACHE;

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

command!(ping(_context, message) {
    message.reply(&format!("Pong! {}", use_emoji(None, "DIDNEYWORL")))?;
});

command!(roll(_context, message, args) {
    lazy_static! {
        static ref DIE_RE: Regex = Regex::new(r"(\d+)?d(\d+)").expect("Invalid DIE_RE");
    }

    let original = if args.is_empty() { "1d6" } else { args.full() };
    let rolled = DIE_RE.replace_all(original, |caps: &Captures| {
        let rolls: usize = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
        let sides: usize = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(6);
        if rolls < 1 {
            String::new()
        } else if sides < 1 {
            "0".to_owned()
        } else {
            let results: Vec<String> = (0..rolls).map(|_| rand::thread_rng().gen_range(1, sides + 1).to_string()).collect();
            results.join(" + ")
        }
    });
    let result = meval::eval_str(&rolled)?;
    let output = format!("{} \u{2192} {} \u{2192} **{}**", original, rolled, result);
    if result.to_string() == rolled || original == rolled || output.len() > CONFIG.discord.long_msg_threshold {
        message.reply(&format!("{} \u{2192} **{}**", original, result))?;
    } else {
        message.reply(&output)?;
    }
});

command!(info(_context, message) {
    let avatar = CACHE.read().user.face();
    message.channel_id.send_message(|msg| {
        msg.embed(|e|
            e.colour(Colour::gold())
            .thumbnail(avatar)
            .field("Author", "<@119122043923988483>", false)
            .field("Source code", "https://gitlab.com/AtteLynx/flutterbitch", false)
            .footer(|f| f.text(&format!("Use {}help for a list of available commands.", CONFIG.discord.command_prefix)))
        )
    })?;
});

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
