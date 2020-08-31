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
