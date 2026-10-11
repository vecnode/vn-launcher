//! Reading GitHub's timestamps and describing how long ago they were.
//!
//! This is pure arithmetic over a fixed format, so it lives in the core with tests
//! rather than pulling in a date library. The clock is never read here: "now" is a
//! parameter, which is exactly what makes the answers testable.

/// Seconds between the Unix epoch and 1970-01-01, per the civil-calendar algorithm.
const SECONDS_PER_DAY: i64 = 86_400;

/// Parse an ISO-8601 timestamp as GitHub writes it, into seconds since the epoch.
///
/// Accepts `2026-03-02T14:23:11Z`, a fractional part, and a `+HH:MM` or `-HH:MM`
/// offset. Returns `None` for anything else — the caller then shows the raw string
/// rather than a wrong time.
#[must_use]
pub fn parse_iso8601(text: &str) -> Option<i64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    // A space is accepted where the standard wants a `T`, because it is a common
    // way to write it and costs nothing to allow.
    if !matches!(bytes[10], b'T' | b't' | b' ') {
        return None;
    }

    let year: i64 = text.get(0..4)?.parse().ok()?;
    let month: u32 = text.get(5..7)?.parse().ok()?;
    let day: u32 = text.get(8..10)?.parse().ok()?;
    let hour: i64 = text.get(11..13)?.parse().ok()?;
    let minute: i64 = text.get(14..16)?.parse().ok()?;
    let second: i64 = text.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let mut seconds =
        days_from_civil(year, month, day) * SECONDS_PER_DAY + hour * 3_600 + minute * 60 + second;

    // Apply a trailing offset, if there is one.
    if let Some(rest) = text.get(19..) {
        let rest = rest.trim_start_matches(['.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
        let (sign, offset) = match rest.as_bytes().first() {
            Some(b'+') => (1, &rest[1..]),
            Some(b'-') => (-1, &rest[1..]),
            // `Z`, an empty tail, or anything unrecognised means UTC.
            _ => (0, ""),
        };
        if sign != 0 {
            let offset_hours: i64 = offset.get(0..2).and_then(|h| h.parse().ok()).unwrap_or(0);
            let offset_minutes: i64 = offset.get(3..5).and_then(|m| m.parse().ok()).unwrap_or(0);
            seconds -= sign * (offset_hours * 3_600 + offset_minutes * 60);
        }
    }

    Some(seconds)
}

/// Describe how long ago something happened, in the fewest useful words.
#[must_use]
pub fn describe_age(then: i64, now: i64) -> String {
    let seconds = now.saturating_sub(then);
    if seconds < 60 {
        return "just now".to_owned();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes} min ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours} h ago");
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{days} d ago");
    }
    let weeks = days / 7;
    if days < 30 {
        return format!("{weeks} w ago");
    }
    let months = days / 30;
    if days < 365 {
        return format!("{months} mo ago");
    }
    format!("{} y ago", days / 365)
}

/// Write a timestamp as a plain date, `2 Mar 2026`.
#[must_use]
pub fn format_date(then: i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days = then.div_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let index = (month.clamp(1, 12) - 1) as usize;
    format!("{day} {} {year}", MONTHS[index])
}

/// Days since the epoch for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted_month = if month > 2 { month - 3 } else { month + 9 } as i64;
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The civil date for a count of days since the epoch.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// 2026-03-02T14:23:11Z, as `date -u -d @<epoch>` would report it.
    const SAMPLE: i64 = 1_772_461_391;

    #[test]
    fn parses_the_shape_github_sends() {
        assert_eq!(parse_iso8601("2026-03-02T14:23:11Z"), Some(SAMPLE));
    }

    #[test]
    fn tolerates_a_space_and_a_fraction() {
        assert_eq!(parse_iso8601("2026-03-02 14:23:11"), Some(SAMPLE));
        assert_eq!(parse_iso8601("2026-03-02T14:23:11.123Z"), Some(SAMPLE));
    }

    #[test]
    fn applies_a_timezone_offset() {
        // 15:23:11+01:00 is the same instant as 14:23:11Z.
        assert_eq!(parse_iso8601("2026-03-02T15:23:11+01:00"), Some(SAMPLE));
        // 09:23:11-05:00 is too.
        assert_eq!(parse_iso8601("2026-03-02T09:23:11-05:00"), Some(SAMPLE));
    }

    #[test]
    fn refuses_anything_it_cannot_read_exactly() {
        for bad in [
            "",
            "yesterday",
            "2026-03-02",
            "2026-03-02T14:23",
            "2026-13-02T14:23:11Z",
            "2026-03-02T25:23:11Z",
        ] {
            assert_eq!(parse_iso8601(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn dates_round_trip_through_the_algorithms() {
        for (year, month, day) in [
            (1970, 1, 1),
            (2000, 2, 29),
            (2024, 12, 31),
            (2026, 3, 2),
            (2100, 3, 1),
        ] {
            let days = days_from_civil(year, month, day);
            assert_eq!(civil_from_days(days), (year, month, day), "{year}-{month}-{day}");
        }
    }

    #[test]
    fn the_epoch_is_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(parse_iso8601("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn ages_read_the_way_people_say_them() {
        let now = SAMPLE;
        assert_eq!(describe_age(now, now), "just now");
        assert_eq!(describe_age(now - 30, now), "just now");
        assert_eq!(describe_age(now - 5 * 60, now), "5 min ago");
        assert_eq!(describe_age(now - 3 * 3_600, now), "3 h ago");
        assert_eq!(describe_age(now - 2 * 86_400, now), "2 d ago");
        assert_eq!(describe_age(now - 15 * 86_400, now), "2 w ago");
        assert_eq!(describe_age(now - 100 * 86_400, now), "3 mo ago");
        assert_eq!(describe_age(now - 800 * 86_400, now), "2 y ago");
    }

    #[test]
    fn a_clock_that_runs_backwards_does_not_produce_nonsense() {
        assert_eq!(describe_age(SAMPLE, SAMPLE - 500), "just now");
    }

    #[test]
    fn dates_are_written_for_a_person() {
        assert_eq!(format_date(SAMPLE), "2 Mar 2026");
        assert_eq!(format_date(0), "1 Jan 1970");
    }
}
