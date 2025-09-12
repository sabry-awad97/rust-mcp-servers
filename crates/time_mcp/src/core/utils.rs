use chrono::DateTime;
use chrono_tz::{OffsetComponents, Tz};

// Constants for format strings and error codes
pub const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%:z";
pub const TIME_INPUT_FORMAT: &str = "%H:%M";
pub const DAY_FORMAT: &str = "%A";

pub fn format_time_difference(hours_difference: f64) -> String {
    if hours_difference.fract() == 0.0 {
        format!("{:+.1}h", hours_difference)
    } else {
        let formatted = format!("{:+.2}", hours_difference);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("{}h", trimmed)
    }
}

pub fn calculate_time_difference(source_time: &DateTime<Tz>, target_time: &DateTime<Tz>) -> String {
    let source_offset = source_time.offset().base_utc_offset() + source_time.offset().dst_offset();
    let target_offset = target_time.offset().dst_offset() + target_time.offset().dst_offset();
    let hours_difference = (target_offset - source_offset).num_seconds() as f64 / 3600.0;

    format_time_difference(hours_difference)
}

#[cfg(test)]
mod tests {
    use super::format_time_difference;

    #[test]
    fn test_format_time_difference() {
        // Test whole hours
        assert_eq!(format_time_difference(5.0), "+5.0h");
        assert_eq!(format_time_difference(-3.0), "-3.0h");

        // Test fractional hours
        assert_eq!(format_time_difference(5.5), "+5.5h");
        assert_eq!(format_time_difference(-2.75), "-2.75h");

        // Test Nepal timezone (UTC+5:45)
        assert_eq!(format_time_difference(5.75), "+5.75h");
    }
}
