use std::str::FromStr;

use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::core::{
    error::{TimeServerError, TimeServerResult},
    models::{TimeConversionResult, TimeResult},
    utils::{self, TIME_INPUT_FORMAT},
};

/// Time server implementation
#[derive(Clone)]
pub struct TimeServer {
    pub(crate) local_timezone: Tz,
}

impl TimeServer {
    pub fn new() -> Self {
        // Try to detect the system's local timezone
        let local_tz = match iana_time_zone::get_timezone() {
            Ok(tz_name) => {
                // Parse the timezone name to get the chrono_tz timezone
                match tz_name.parse::<chrono_tz::Tz>() {
                    Ok(tz) => tz,
                    Err(_) => {
                        tracing::warn!("Could not parse timezone '{}', defaulting to UTC", tz_name);
                        chrono_tz::UTC
                    }
                }
            }
            Err(_) => {
                tracing::warn!("Could not detect system timezone, defaulting to UTC");
                chrono_tz::UTC
            }
        };

        Self {
            local_timezone: local_tz,
        }
    }

    pub(crate) fn parse_timezone(&self, timezone_name: &str) -> TimeServerResult<Tz> {
        Tz::from_str(timezone_name).map_err(|_| TimeServerError::InvalidTimezone {
            timezone: timezone_name.to_string(),
        })
    }

    pub fn get_current_time(&self, timezone_name: &str) -> TimeServerResult<TimeResult> {
        let timezone = self.parse_timezone(timezone_name)?;
        let current_time = Utc::now().with_timezone(&timezone);

        Ok(TimeResult::from_datetime(&current_time, timezone_name))
    }

    pub fn convert_time(
        &self,
        source_tz: &str,
        time_str: &str,
        target_tz: &str,
    ) -> TimeServerResult<TimeConversionResult> {
        let source_timezone = self.parse_timezone(source_tz)?;
        let target_timezone = self.parse_timezone(target_tz)?;

        let (source_time, target_time) =
            self.perform_time_conversion(&source_timezone, time_str, &target_timezone)?;

        let time_difference = utils::calculate_time_difference(&source_time, &target_time);

        Ok(TimeConversionResult {
            source: TimeResult::from_datetime(&source_time, source_tz),
            target: TimeResult::from_datetime(&target_time, target_tz),
            time_difference,
        })
    }

    fn perform_time_conversion(
        &self,
        source_tz: &Tz,
        time_str: &str,
        target_tz: &Tz,
    ) -> TimeServerResult<(DateTime<Tz>, DateTime<Tz>)> {
        let parsed_time = NaiveTime::parse_from_str(time_str, TIME_INPUT_FORMAT).map_err(|_| {
            TimeServerError::InvalidTimeFormat {
                time: time_str.to_string(),
            }
        })?;

        let now = Utc::now().with_timezone(source_tz);
        let source_time = source_tz
            .from_local_datetime(&now.date_naive().and_time(parsed_time))
            .single()
            .ok_or_else(|| TimeServerError::AmbiguousTime {
                time: time_str.to_string(),
            })?;

        let target_time = source_time.with_timezone(target_tz);
        Ok((source_time, target_time))
    }
}

impl Default for TimeServer {
    fn default() -> Self {
        Self::new()
    }
}
