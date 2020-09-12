use itertools::Itertools;
use std::time::Duration;

pub fn ellipsis_string(s: impl AsRef<str>, len: usize) -> String {
    let s = s.as_ref();
    if s.chars().count() > len {
        format!(
            "{}\u{2026}", // ellipsis
            s.chars().take(len - 1).collect::<String>()
        )
    } else {
        s.to_owned()
    }
}

pub fn separate_thousands(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let mut chars = s.chars();
    let mut prefix = chars.by_ref().take(s.len() % 3).collect::<String>();
    let rest = chars
        .chunks(3)
        .into_iter()
        .map(Iterator::collect::<String>)
        .join(" ");
    if prefix.is_empty() {
        rest
    } else if rest.is_empty() {
        prefix
    } else {
        prefix.push(' ');
        prefix.push_str(&rest);
        prefix
    }
}

pub fn format_duration(duration: &Duration) -> String {
    let hours = duration.as_secs() / 60 / 60;
    let mins = (duration.as_secs() / 60) % 60;
    let secs = duration.as_secs() % 60;

    let mut out = String::new();
    if hours > 0 {
        out.push_str(&hours.to_string());
        out.push_str(" hours");
    }
    if mins > 0 {
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&mins.to_string());
        out.push_str(" minutes");
    }
    if secs > 0 {
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&secs.to_string());
        out.push_str(" seconds");
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_separate_thousands() {
        use super::separate_thousands;
        assert_eq!(separate_thousands("1"), "1");
        assert_eq!(separate_thousands("12"), "12");
        assert_eq!(separate_thousands("123"), "123");
        assert_eq!(separate_thousands("1234"), "1 234");
        assert_eq!(separate_thousands("12345"), "12 345");
        assert_eq!(separate_thousands("123456"), "123 456");
        assert_eq!(separate_thousands("1234567"), "1 234 567");
        assert_eq!(separate_thousands("12345678"), "12 345 678");
        assert_eq!(separate_thousands("123456789"), "123 456 789");
    }
}
