//! time parsing utilities for conditions
//!
//! supports formats:
//! - time ranges: "09:00-17:00", "9:00AM-5:00PM", "9AM-5PM"
//! - multiple ranges: "09:00-12:00,14:00-18:00"
//! - overnight ranges: "22:00-06:00" (automatically handled)
//! - day specs: "mon", "mon-fri", "mon,wed,fri", "mon-wed,fri,sun"

use chrono::{Datelike, Local, Timelike, Weekday};

/// a time range in minutes from midnight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// start time in minutes from midnight (0-1439)
    pub start: u16,
    /// end time in minutes from midnight (0-1439)
    pub end: u16,
}

impl TimeRange {
    /// create a new time range
    pub fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// check if a time (in minutes from midnight) is within this range
    /// handles overnight ranges automatically
    pub fn contains(&self, minutes: u16) -> bool {
        if self.start <= self.end {
            // normal range: 09:00-17:00
            minutes >= self.start && minutes < self.end
        } else {
            // overnight range: 22:00-06:00
            minutes >= self.start || minutes < self.end
        }
    }

    /// check if current time is within this range
    #[allow(dead_code)]
    pub fn is_now(&self) -> bool {
        let now = Local::now();
        let minutes = (now.hour() * 60 + now.minute()) as u16;
        self.contains(minutes)
    }
}

/// parse a time string into minutes from midnight
/// supports: "09:00", "9:00", "9:00AM", "9AM", "17:00", "5:00PM", "5PM"
pub fn parse_time(s: &str) -> Option<u16> {
    let s = s.trim().to_uppercase();

    // check for AM/PM suffix
    let (time_part, is_pm, is_am) = if s.ends_with("AM") {
        (s.trim_end_matches("AM").trim(), false, true)
    } else if s.ends_with("PM") {
        (s.trim_end_matches("PM").trim(), true, false)
    } else {
        (s.as_str(), false, false)
    };

    // parse hour and optional minute
    let (hour, minute) = if time_part.contains(':') {
        let parts: Vec<&str> = time_part.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let h: u16 = parts[0].parse().ok()?;
        let m: u16 = parts[1].parse().ok()?;
        (h, m)
    } else {
        // just hour: "9AM", "17"
        let h: u16 = time_part.parse().ok()?;
        (h, 0)
    };

    // validate
    if minute >= 60 {
        return None;
    }

    // convert to 24h format
    let hour_24 = if is_pm {
        if hour == 12 {
            12
        } else {
            hour + 12
        }
    } else if is_am {
        if hour == 12 {
            0
        } else {
            hour
        }
    } else {
        hour
    };

    if hour_24 >= 24 {
        return None;
    }

    Some(hour_24 * 60 + minute)
}

/// parse a time range string like "09:00-17:00" or "9AM-5PM"
pub fn parse_time_range(s: &str) -> Option<TimeRange> {
    let s = s.trim();

    // split on dash, but be careful with negative numbers (not applicable here)
    let parts: Vec<&str> = s.split('-').collect();

    // handle formats like "9AM-5PM" where there's one dash
    // and "09:00-17:00" where there's one dash
    if parts.len() == 2 {
        let start = parse_time(parts[0])?;
        let end = parse_time(parts[1])?;
        return Some(TimeRange::new(start, end));
    }

    // handle "9:00AM-5:00PM" which might split differently
    // try to find the separator dash between two time values
    if let Some(dash_pos) = find_range_separator(s) {
        let start = parse_time(&s[..dash_pos])?;
        let end = parse_time(&s[dash_pos + 1..])?;
        return Some(TimeRange::new(start, end));
    }

    None
}

/// find the position of the dash that separates two times
fn find_range_separator(s: &str) -> Option<usize> {
    // look for a dash that's not part of a time format
    // times can have colons, so look for dash after a digit or AM/PM
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'-' && i > 0 {
            let prev = bytes[i - 1];
            // dash after digit, M (from AM/PM), or space is a separator
            if prev.is_ascii_digit() || prev == b'M' || prev == b'm' || prev == b' ' {
                // make sure there's something after
                if i + 1 < bytes.len() {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// parse multiple time ranges separated by commas
/// e.g., "09:00-12:00,14:00-18:00"
pub fn parse_time_ranges(s: &str) -> Option<Vec<TimeRange>> {
    let ranges: Vec<TimeRange> = s
        .split(',')
        .filter_map(|part| parse_time_range(part.trim()))
        .collect();

    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

/// check if current time is within any of the given ranges
pub fn is_time_in_ranges(ranges: &[TimeRange]) -> bool {
    let now = Local::now();
    let minutes = (now.hour() * 60 + now.minute()) as u16;
    ranges.iter().any(|r| r.contains(minutes))
}

/// parse a weekday from string
pub fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// get the next weekday in sequence
fn next_weekday(day: Weekday) -> Weekday {
    match day {
        Weekday::Mon => Weekday::Tue,
        Weekday::Tue => Weekday::Wed,
        Weekday::Wed => Weekday::Thu,
        Weekday::Thu => Weekday::Fri,
        Weekday::Fri => Weekday::Sat,
        Weekday::Sat => Weekday::Sun,
        Weekday::Sun => Weekday::Mon,
    }
}

/// expand a day range like "mon-fri" into a list of weekdays
fn expand_day_range(start: Weekday, end: Weekday) -> Vec<Weekday> {
    let mut days = vec![start];
    let mut current = start;

    while current != end {
        current = next_weekday(current);
        days.push(current);
    }

    days
}

/// parse a day specification
/// supports: "mon", "mon-fri", "mon,wed,fri", "mon-wed,fri,sun"
pub fn parse_days(s: &str) -> Option<Vec<Weekday>> {
    let s = s.trim().to_lowercase();

    let mut days = Vec::new();

    for part in s.split(',') {
        let part = part.trim();

        if part.contains('-') {
            // range: "mon-fri"
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return None;
            }
            let start = parse_weekday(range_parts[0].trim())?;
            let end = parse_weekday(range_parts[1].trim())?;
            days.extend(expand_day_range(start, end));
        } else {
            // single day: "mon"
            days.push(parse_weekday(part)?);
        }
    }

    if days.is_empty() {
        None
    } else {
        // deduplicate while preserving order
        let mut seen = std::collections::HashSet::new();
        days.retain(|d| seen.insert(*d));
        Some(days)
    }
}

/// check if current day matches any of the given weekdays
pub fn is_day_match(days: &[Weekday]) -> bool {
    let today = Local::now().weekday();
    days.contains(&today)
}

/// check if current day matches a day specification string
pub fn is_day_spec_match(spec: &str) -> bool {
    parse_days(spec)
        .map(|days| is_day_match(&days))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_24h() {
        assert_eq!(parse_time("09:00"), Some(9 * 60));
        assert_eq!(parse_time("9:00"), Some(9 * 60));
        assert_eq!(parse_time("17:00"), Some(17 * 60));
        assert_eq!(parse_time("00:00"), Some(0));
        assert_eq!(parse_time("23:59"), Some(23 * 60 + 59));
        assert_eq!(parse_time("12:30"), Some(12 * 60 + 30));
    }

    #[test]
    fn test_parse_time_ampm() {
        assert_eq!(parse_time("9:00AM"), Some(9 * 60));
        assert_eq!(parse_time("9AM"), Some(9 * 60));
        assert_eq!(parse_time("9:00am"), Some(9 * 60));
        assert_eq!(parse_time("12:00PM"), Some(12 * 60));
        assert_eq!(parse_time("12PM"), Some(12 * 60));
        assert_eq!(parse_time("5:00PM"), Some(17 * 60));
        assert_eq!(parse_time("5PM"), Some(17 * 60));
        assert_eq!(parse_time("12:00AM"), Some(0));
        assert_eq!(parse_time("12AM"), Some(0));
    }

    #[test]
    fn test_parse_time_invalid() {
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("12:60"), None);
        assert_eq!(parse_time("invalid"), None);
    }

    #[test]
    fn test_parse_time_range() {
        let r = parse_time_range("09:00-17:00").unwrap();
        assert_eq!(r.start, 9 * 60);
        assert_eq!(r.end, 17 * 60);

        let r = parse_time_range("9AM-5PM").unwrap();
        assert_eq!(r.start, 9 * 60);
        assert_eq!(r.end, 17 * 60);

        let r = parse_time_range("9:00AM-5:00PM").unwrap();
        assert_eq!(r.start, 9 * 60);
        assert_eq!(r.end, 17 * 60);
    }

    #[test]
    fn test_time_range_contains() {
        // normal range
        let r = TimeRange::new(9 * 60, 17 * 60);
        assert!(r.contains(9 * 60)); // 09:00 - start inclusive
        assert!(r.contains(12 * 60)); // 12:00 - middle
        assert!(!r.contains(17 * 60)); // 17:00 - end exclusive
        assert!(!r.contains(8 * 60)); // 08:00 - before
        assert!(!r.contains(18 * 60)); // 18:00 - after

        // overnight range
        let r = TimeRange::new(22 * 60, 6 * 60);
        assert!(r.contains(22 * 60)); // 22:00 - start
        assert!(r.contains(23 * 60)); // 23:00 - evening
        assert!(r.contains(0)); // 00:00 - midnight
        assert!(r.contains(5 * 60)); // 05:00 - early morning
        assert!(!r.contains(6 * 60)); // 06:00 - end exclusive
        assert!(!r.contains(12 * 60)); // 12:00 - daytime
    }

    #[test]
    fn test_parse_time_ranges() {
        let ranges = parse_time_ranges("09:00-12:00,14:00-18:00").unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 9 * 60);
        assert_eq!(ranges[0].end, 12 * 60);
        assert_eq!(ranges[1].start, 14 * 60);
        assert_eq!(ranges[1].end, 18 * 60);

        let ranges = parse_time_ranges("9AM-12PM, 2PM-6PM").unwrap();
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("mon"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("monday"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("Mon"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("MONDAY"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("fri"), Some(Weekday::Fri));
        assert_eq!(parse_weekday("invalid"), None);
    }

    #[test]
    fn test_parse_days_single() {
        let days = parse_days("mon").unwrap();
        assert_eq!(days, vec![Weekday::Mon]);
    }

    #[test]
    fn test_parse_days_list() {
        let days = parse_days("mon,wed,fri").unwrap();
        assert_eq!(days, vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]);
    }

    #[test]
    fn test_parse_days_range() {
        let days = parse_days("mon-fri").unwrap();
        assert_eq!(
            days,
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri
            ]
        );

        let days = parse_days("sat-sun").unwrap();
        assert_eq!(days, vec![Weekday::Sat, Weekday::Sun]);
    }

    #[test]
    fn test_parse_days_mixed() {
        let days = parse_days("mon-wed,fri,sun").unwrap();
        assert_eq!(
            days,
            vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Fri,
                Weekday::Sun
            ]
        );
    }

    #[test]
    fn test_parse_days_wrap_around() {
        // fri-mon should give fri, sat, sun, mon
        let days = parse_days("fri-mon").unwrap();
        assert_eq!(
            days,
            vec![Weekday::Fri, Weekday::Sat, Weekday::Sun, Weekday::Mon]
        );
    }
}
