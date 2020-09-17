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

fn separate_thousands_impl(s: impl AsRef<str>, reverse: bool) -> String {
    let s = s.as_ref();
    let mut chars = s.chars();
    let mut prefix = if reverse {
        String::new()
    } else {
        chars.by_ref().take(s.len() % 3).collect::<String>()
    };
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

#[inline]
pub fn separate_thousands_unsigned(n: usize) -> String {
    separate_thousands_impl(n.to_string(), false)
}

pub fn separate_thousands_signed(n: isize) -> String {
    if n < 0 {
        format!("-{}", separate_thousands_impl(n.abs().to_string(), false))
    } else {
        separate_thousands_impl(n.to_string(), false)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn separate_thousands_floating(n: f64) -> String {
    if !n.is_normal() {
        n.to_string()
    } else if n.abs().fract() <= f64::EPSILON {
        separate_thousands_signed(n.trunc() as isize)
    } else {
        format!(
            "{}{}.{}",
            if n < 0.0 { "-" } else { "" },
            separate_thousands_unsigned(n.abs().trunc() as usize),
            separate_thousands_impl(&n.abs().fract().to_string()[2..], true),
        )
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
    use super::*;

    #[test]
    fn test_separate_thousands_unsigned() {
        assert_eq!(separate_thousands_unsigned(0), "0");
        assert_eq!(separate_thousands_unsigned(1), "1");
        assert_eq!(separate_thousands_unsigned(10), "10");
        assert_eq!(separate_thousands_unsigned(100), "100");
        assert_eq!(separate_thousands_unsigned(1_000), "1 000");
        assert_eq!(separate_thousands_unsigned(10_000), "10 000");
        assert_eq!(separate_thousands_unsigned(100_000), "100 000");
        assert_eq!(separate_thousands_unsigned(1_000_000), "1 000 000");
        assert_eq!(separate_thousands_unsigned(10_000_000), "10 000 000");
    }

    #[test]
    fn test_separate_thousands_signed() {
        assert_eq!(separate_thousands_signed(0), "0");
        assert_eq!(separate_thousands_signed(1), "1");
        assert_eq!(separate_thousands_signed(10), "10");
        assert_eq!(separate_thousands_signed(100), "100");
        assert_eq!(separate_thousands_signed(1_000), "1 000");
        assert_eq!(separate_thousands_signed(10_000), "10 000");
        assert_eq!(separate_thousands_signed(100_000), "100 000");
        assert_eq!(separate_thousands_signed(1_000_000), "1 000 000");
        assert_eq!(separate_thousands_signed(10_000_000), "10 000 000");

        assert_eq!(separate_thousands_signed(-0), "0");
        assert_eq!(separate_thousands_signed(-1), "-1");
        assert_eq!(separate_thousands_signed(-10), "-10");
        assert_eq!(separate_thousands_signed(-100), "-100");
        assert_eq!(separate_thousands_signed(-1_000), "-1 000");
        assert_eq!(separate_thousands_signed(-10_000), "-10 000");
        assert_eq!(separate_thousands_signed(-100_000), "-100 000");
        assert_eq!(separate_thousands_signed(-1_000_000), "-1 000 000");
        assert_eq!(separate_thousands_signed(-10_000_000), "-10 000 000");
    }

    #[test]
    fn test_separate_thousands_floating() {
        assert_eq!(separate_thousands_floating(f64::NAN), "NaN");
        assert_eq!(separate_thousands_floating(f64::INFINITY), "inf");
        assert_eq!(separate_thousands_floating(f64::NEG_INFINITY), "-inf");

        assert_eq!(separate_thousands_floating(0.0), "0");
        assert_eq!(separate_thousands_floating(-0.0), "0");
        assert_eq!(separate_thousands_floating(1.0), "1");
        assert_eq!(separate_thousands_floating(-1.0), "-1");

        assert_eq!(separate_thousands_floating(-0.011_718_75), "-0.011 718 75");
    }
}
