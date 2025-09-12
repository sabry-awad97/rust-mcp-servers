use crate::core::error::{SleepServerError, SleepServerResult};
use std::time::Duration;

/// Maximum allowed sleep duration (30 minutes)
pub const MAX_SLEEP_DURATION: Duration = Duration::from_secs(30 * 60);

/// Parse a duration string into a Duration
/// Supports formats like: "1s", "500ms", "2m", "1h", "1.5s"
pub fn parse_duration(duration_str: &str) -> SleepServerResult<Duration> {
    let duration_str = duration_str.trim();

    if duration_str.is_empty() {
        return Err(SleepServerError::InvalidDuration {
            duration: duration_str.to_string(),
        });
    }

    // Find where the number ends and unit begins
    let mut split_pos = 0;
    for (i, c) in duration_str.char_indices() {
        if c.is_alphabetic() {
            split_pos = i;
            break;
        }
    }

    if split_pos == 0 {
        return Err(SleepServerError::InvalidDuration {
            duration: duration_str.to_string(),
        });
    }

    let (number_part, unit_part) = duration_str.split_at(split_pos);

    let number: f64 = number_part
        .parse()
        .map_err(|_| SleepServerError::InvalidDuration {
            duration: duration_str.to_string(),
        })?;

    if number < 0.0 {
        return Err(SleepServerError::InvalidDuration {
            duration: duration_str.to_string(),
        });
    }

    let duration = match unit_part.to_lowercase().as_str() {
        "ms" | "milliseconds" => Duration::from_millis((number) as u64),
        "s" | "sec" | "seconds" => Duration::from_millis((number * 1000.0) as u64),
        "m" | "min" | "minutes" => Duration::from_millis((number * 60.0 * 1000.0) as u64),
        "h" | "hr" | "hours" => Duration::from_millis((number * 60.0 * 60.0 * 1000.0) as u64),
        _ => {
            return Err(SleepServerError::InvalidDuration {
                duration: duration_str.to_string(),
            })
        }
    };

    if duration > MAX_SLEEP_DURATION {
        return Err(SleepServerError::DurationTooLong {
            duration: duration_str.to_string(),
            max_duration: format_duration(MAX_SLEEP_DURATION),
        });
    }

    Ok(duration)
}

/// Format a duration into a human-readable string
pub fn format_duration(duration: Duration) -> String {
    let total_ms = duration.as_millis() as u64;

    if total_ms == 0 {
        return "0ms".to_string();
    }

    let hours = total_ms / (1000 * 60 * 60);
    let minutes = (total_ms % (1000 * 60 * 60)) / (1000 * 60);
    let seconds = (total_ms % (1000 * 60)) / 1000;
    let milliseconds = total_ms % 1000;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 {
        if milliseconds > 0 {
            parts.push(format!("{}.{:03}s", seconds, milliseconds));
        } else {
            parts.push(format!("{}s", seconds));
        }
    } else if milliseconds > 0 {
        parts.push(format!("{}ms", milliseconds));
    }

    if parts.is_empty() {
        "0ms".to_string()
    } else {
        parts.join(" ")
    }
}

/// Calculate progress percentage for a sleep operation
pub fn calculate_progress(elapsed: Duration, total: Duration) -> f64 {
    if total.as_millis() == 0 {
        return 100.0;
    }

    let progress = (elapsed.as_millis() as f64 / total.as_millis() as f64) * 100.0;
    progress.clamp(0.0, 100.0)
}

/// Parse an ISO 8601 timestamp
pub fn parse_iso8601(timestamp: &str) -> SleepServerResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| SleepServerError::InvalidDuration {
            duration: format!("Invalid timestamp: {}", timestamp),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse_duration("0.1s").unwrap(), Duration::from_millis(100));
    }

    #[test]
    fn test_parse_duration_milliseconds() {
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1000ms").unwrap(), Duration::from_secs(1));
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("1.5m").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn test_parse_duration_hours() {
        // Test parsing hours that are within the 30-minute limit
        assert_eq!(parse_duration("0.25h").unwrap(), Duration::from_secs(900)); // 15 minutes
        assert_eq!(parse_duration("0.5h").unwrap(), Duration::from_secs(1800)); // 30 minutes
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("5").is_err());
        assert!(parse_duration("-1s").is_err());
    }

    #[test]
    fn test_parse_duration_too_long() {
        // Test duration longer than 30 minutes
        assert!(parse_duration("31m").is_err());
        assert!(parse_duration("1h").is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.500s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    #[test]
    fn test_calculate_progress() {
        assert_eq!(
            calculate_progress(Duration::from_secs(0), Duration::from_secs(10)),
            0.0
        );
        assert_eq!(
            calculate_progress(Duration::from_secs(5), Duration::from_secs(10)),
            50.0
        );
        assert_eq!(
            calculate_progress(Duration::from_secs(10), Duration::from_secs(10)),
            100.0
        );
        assert_eq!(
            calculate_progress(Duration::from_secs(15), Duration::from_secs(10)),
            100.0
        );
    }

    #[test]
    fn test_parse_iso8601() {
        let timestamp = "2025-01-15T14:30:00Z";
        let parsed = parse_iso8601(timestamp).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2025-01-15T14:30:00+00:00");
    }
}
