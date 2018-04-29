use super::super::util::use_emoji;
use meval;
use rand::{self, Rng};
use regex::{Captures, Regex};

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
        let results: Vec<String> = (0..rolls).map(|_| rand::thread_rng().gen_range(1usize, sides + 1).to_string()).collect();
        results.join(" + ")
    });
    let result = meval::eval_str(&rolled)?;
    if result.to_string() == rolled {
        message.reply(&format!("{} \u{2192} {}", original, rolled))?;
    } else {
        message.reply(&format!("{} \u{2192} {} \u{2192} {}", original, rolled, result))?;
    }
});
