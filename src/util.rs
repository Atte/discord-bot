use itertools::Itertools;

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
