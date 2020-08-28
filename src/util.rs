pub fn ellipsis_string(s: String, len: usize) -> String {
    if s.chars().count() > len {
        format!("{}\u{2026}", s.chars().take(len - 1).collect::<String>())
    } else {
        s
    }
}
