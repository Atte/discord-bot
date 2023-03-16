#![allow(dead_code)] // which utilities get used depends on crate features

use conv::ConvUtil;
use itertools::Itertools;
use std::time::Duration;

const ELLIPSIS: char = '\u{2026}';

pub fn ellipsis_string(s: impl AsRef<str>, len: usize) -> String {
    let s = s.as_ref();
    if len == 0 {
        String::new()
    } else if s.chars().count() > len {
        format!("{}{ELLIPSIS}", s.chars().take(len - 1).collect::<String>())
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

pub fn separate_thousands_floating(n: f64) -> String {
    if n == 0.0 || n == -0.0 {
        "0".to_owned()
    } else if !n.is_normal() {
        n.to_string()
    } else if n.abs().fract() <= f64::EPSILON {
        n.trunc()
            .approx_as::<isize>()
            .map_or_else(|err| err.to_string(), separate_thousands_signed)
    } else {
        n.abs().trunc().approx_as::<usize>().map_or_else(
            |err| err.to_string(),
            |trunc| {
                format!(
                    "{}{}.{}",
                    if n.is_sign_negative() { "-" } else { "" },
                    separate_thousands_unsigned(trunc),
                    separate_thousands_impl(&n.abs().fract().to_string()[2..], true),
                )
            },
        )
    }
}

pub fn format_duration_long(duration: &Duration) -> String {
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
    if secs > 0 || out.is_empty() {
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&secs.to_string());
        out.push_str(" seconds");
    }
    out
}

pub fn format_duration_short(duration: &Duration) -> String {
    let hours = duration.as_secs() / 60 / 60;
    let mins = (duration.as_secs() / 60) % 60;
    let secs = duration.as_secs() % 60;

    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::ELLIPSIS;
    use std::time::Duration;

    #[test]
    fn ellipsis_string() {
        assert_eq!(super::ellipsis_string("testing", 0), "");
        assert_eq!(
            super::ellipsis_string("testing", 5),
            format!("test{ELLIPSIS}")
        );
        assert_eq!(super::ellipsis_string("testing", 50), "testing");
    }

    #[test]
    fn separate_thousands_unsigned() {
        assert_eq!(super::separate_thousands_unsigned(0), "0");
        assert_eq!(super::separate_thousands_unsigned(1), "1");
        assert_eq!(super::separate_thousands_unsigned(10), "10");
        assert_eq!(super::separate_thousands_unsigned(100), "100");
        assert_eq!(super::separate_thousands_unsigned(1_000), "1 000");
        assert_eq!(super::separate_thousands_unsigned(10_000), "10 000");
        assert_eq!(super::separate_thousands_unsigned(100_000), "100 000");
        assert_eq!(super::separate_thousands_unsigned(1_000_000), "1 000 000");
        assert_eq!(super::separate_thousands_unsigned(10_000_000), "10 000 000");
    }

    #[test]
    fn separate_thousands_signed() {
        assert_eq!(super::separate_thousands_signed(0), "0");
        assert_eq!(super::separate_thousands_signed(1), "1");
        assert_eq!(super::separate_thousands_signed(10), "10");
        assert_eq!(super::separate_thousands_signed(100), "100");
        assert_eq!(super::separate_thousands_signed(1_000), "1 000");
        assert_eq!(super::separate_thousands_signed(10_000), "10 000");
        assert_eq!(super::separate_thousands_signed(100_000), "100 000");
        assert_eq!(super::separate_thousands_signed(1_000_000), "1 000 000");
        assert_eq!(super::separate_thousands_signed(10_000_000), "10 000 000");

        assert_eq!(super::separate_thousands_signed(-0), "0");
        assert_eq!(super::separate_thousands_signed(-1), "-1");
        assert_eq!(super::separate_thousands_signed(-10), "-10");
        assert_eq!(super::separate_thousands_signed(-100), "-100");
        assert_eq!(super::separate_thousands_signed(-1_000), "-1 000");
        assert_eq!(super::separate_thousands_signed(-10_000), "-10 000");
        assert_eq!(super::separate_thousands_signed(-100_000), "-100 000");
        assert_eq!(super::separate_thousands_signed(-1_000_000), "-1 000 000");
        assert_eq!(super::separate_thousands_signed(-10_000_000), "-10 000 000");
    }

    #[test]
    fn separate_thousands_floating() {
        assert_eq!(super::separate_thousands_floating(f64::NAN), "NaN");
        assert_eq!(super::separate_thousands_floating(f64::INFINITY), "inf");
        assert_eq!(
            super::separate_thousands_floating(f64::NEG_INFINITY),
            "-inf"
        );

        assert_eq!(super::separate_thousands_floating(0.0), "0");
        assert_eq!(super::separate_thousands_floating(-0.0), "0");
        assert_eq!(super::separate_thousands_floating(1.0), "1");
        assert_eq!(super::separate_thousands_floating(-1.0), "-1");

        assert_eq!(
            super::separate_thousands_floating(-0.011_718_75),
            "-0.011 718 75"
        );
    }

    #[test]
    fn format_duration() {
        let duration = Duration::from_secs(0);
        assert_eq!(super::format_duration_long(&duration), "0 seconds");
        assert_eq!(super::format_duration_short(&duration), "0:00");

        let duration = Duration::from_secs(4 * 60 + 20);
        assert_eq!(
            super::format_duration_long(&duration),
            "4 minutes, 20 seconds"
        );
        assert_eq!(super::format_duration_short(&duration), "4:20");

        let duration = Duration::from_secs(7 * 60 * 60 + 4 * 60 + 20);
        assert_eq!(
            super::format_duration_long(&duration),
            "7 hours, 4 minutes, 20 seconds"
        );
        assert_eq!(super::format_duration_short(&duration), "7:04:20");

        let duration = Duration::from_secs(4 * 60);
        assert_eq!(super::format_duration_long(&duration), "4 minutes");
        assert_eq!(super::format_duration_short(&duration), "4:00");
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use std::time::Duration;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        #[test]
        fn ellipsis_string_no_change(
            (s, len) in any::<String>().prop_flat_map(|s| {
                let len = s.chars().count();
                (Just(s), len..)
            })
        ) {
            let out = super::ellipsis_string(&s, len);
            assert_eq!(s, out);
        }

        #[test]
        fn ellipsis_string_shorten(
            (s, len) in any::<String>()
                .prop_filter("empty string can't be shortened", |s| !s.is_empty())
                .prop_flat_map(|s| {
                    let len = s.chars().count();
                    (Just(s), ..len)
                })
        ) {
            let out = super::ellipsis_string(s, len);
            assert!(out.chars().count() == len);
            assert!(len == 0 || out.ends_with(super::ELLIPSIS));
        }

        #[test]
        fn separate_thousands_unsigned(s in r"[1-9][0-9]{0,2}( [0-9]{3}){0,6}") {
            match s.replace(' ', "").parse() {
                Ok(n) => assert_eq!(s, super::separate_thousands_unsigned(n)),
                Err(e) => return Err(TestCaseError::reject(e.to_string())),
            }
        }

        #[test]
        fn separate_thousands_signed(s in r"-?[1-9][0-9]{0,2}( [0-9]{3}){0,6}") {
            match s.replace(' ', "").parse() {
                Ok(n) => assert_eq!(s, super::separate_thousands_signed(n)),
                Err(e) => return Err(TestCaseError::reject(e.to_string())),
            }
        }

        #[test]
        fn format_duration_long_seconds(seconds in ..60_u64) {
            let out = super::format_duration_long(&Duration::from_secs(seconds));
            assert_eq!(format!("{seconds} seconds"), out);
        }
        #[test]
        fn format_duration_long_minutes(minutes in 1_u64..60_u64) {
            let out = super::format_duration_long(&Duration::from_secs(minutes * 60));
            assert_eq!(format!("{minutes} minutes"), out);
        }
        #[test]
        fn format_duration_long_hours(hours in 1_u64..=(u64::MAX / 60 / 60)) {
            let out = super::format_duration_long(&Duration::from_secs(hours * 60 * 60));
            assert_eq!(format!("{hours} hours"), out);
        }
        #[test]
        fn format_duration_long_all(seconds in 1_u64..60_u64, minutes in 1_u64..60_u64, hours in 1_u64..=(u64::MAX / 60 / 60)) {
            let out = super::format_duration_long(&Duration::from_secs(seconds + minutes * 60 + hours * 60 * 60));
            assert_eq!(format!("{hours} hours, {minutes} minutes, {seconds} seconds"), out);
        }

        #[test]
        fn format_duration_short_minutes_seconds(seconds in ..60_u64, minutes in ..60_u64) {
            let out = super::format_duration_short(&Duration::from_secs(seconds + minutes * 60));
            assert_eq!(format!("{minutes}:{seconds:02}"), out);
        }
        #[test]
        fn format_duration_short_all(seconds in ..60_u64, minutes in ..60_u64, hours in 1_u64..=(u64::MAX / 60 / 60)) {
            let out = super::format_duration_short(&Duration::from_secs(seconds + minutes * 60 + hours * 60 * 60));
            assert_eq!(format!("{hours}:{minutes:02}:{seconds:02}"), out);
        }
    }
}
