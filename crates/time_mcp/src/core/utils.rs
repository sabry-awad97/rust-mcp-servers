use chrono::DateTime;
use chrono_tz::{OffsetComponents, Tz};

// Constants for format strings and error codes
pub const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%:z";
pub const TIME_INPUT_FORMAT: &str = "%H:%M";
pub const DAY_FORMAT: &str = "%A";

/// Available resource URIs for the Time MCP Server
pub const AVAILABLE_RESOURCES: &[&str] = &["time://status", "time://help", "time://timezones"];

/// Format a time difference in hours
///
/// # Arguments
///
/// * `hours_difference` - The time difference in hours
///
/// # Returns
///
/// A formatted string representing the time difference
pub fn format_time_difference(hours_difference: f64) -> String {
    match hours_difference.fract() {
        0.0 => format!("{:+.0}h", hours_difference),
        _ => {
            let formatted = format!("{:+}", hours_difference);
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            format!("{}h", trimmed)
        }
    }
}

/// Calculate the time difference between two timezones
///
/// # Arguments
///
/// * `source_time` - The source time
/// * `target_time` - The target time
///
/// # Returns
///
/// A formatted string representing the time difference
pub fn calculate_time_difference(source_time: &DateTime<Tz>, target_time: &DateTime<Tz>) -> String {
    let source_offset = source_time.offset().base_utc_offset() + source_time.offset().dst_offset();
    let target_offset = target_time.offset().base_utc_offset() + target_time.offset().dst_offset();
    let hours_difference = (target_offset - source_offset).num_seconds() as f64 / 3600.0;

    format_time_difference(hours_difference)
}

#[cfg(test)]
mod tests {
    use super::format_time_difference;

    #[test]
    fn test_format_time_difference() {
        // Test whole hours (now formatted without decimal)
        assert_eq!(format_time_difference(5.0), "+5h");
        assert_eq!(format_time_difference(-3.0), "-3h");

        // Test fractional hours
        assert_eq!(format_time_difference(5.5), "+5.5h");
        assert_eq!(format_time_difference(-2.75), "-2.75h");

        // Test Nepal timezone (UTC+5:45)
        assert_eq!(format_time_difference(5.75), "+5.75h");
    }
}
