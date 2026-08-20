use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::DomainError;

const NANOS_PER_SECOND: u32 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;

/// A deep-time instant independent of database timestamp ranges.
///
/// Seconds are counted from 1970-01-01T00:00:00 in the versioned HistoryWorld
/// world-clock scale. Astronomical year numbering is used by calendar helpers:
/// year 0 is 1 BCE, year -1 is 2 BCE. APIs encode seconds as a decimal string so
/// JavaScript clients cannot silently lose integer precision.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldInstant {
    seconds: i64,
    nanos: u32,
}

impl WorldInstant {
    pub const UNIX_EPOCH: Self = Self {
        seconds: 0,
        nanos: 0,
    };

    pub fn new(seconds: i64, nanos: u32) -> Result<Self, DomainError> {
        if nanos >= NANOS_PER_SECOND {
            return Err(DomainError::InvalidNanoseconds(nanos));
        }
        Ok(Self { seconds, nanos })
    }

    pub const fn from_seconds(seconds: i64) -> Self {
        Self { seconds, nanos: 0 }
    }

    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    pub const fn nanos(self) -> u32 {
        self.nanos
    }

    /// Constructs an instant from a proleptic Gregorian date.
    pub fn from_ymd_hms(
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, DomainError> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(DomainError::InvalidDate { year, month, day });
        }
        if hour > 23 || minute > 59 || second > 59 {
            return Err(DomainError::InvalidTimeOfDay {
                hour,
                minute,
                second,
            });
        }

        let days = days_from_civil(year, month, day).ok_or(DomainError::TimeOverflow)?;
        let day_seconds = i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
        let seconds = days
            .checked_mul(SECONDS_PER_DAY)
            .and_then(|value| value.checked_add(day_seconds))
            .ok_or(DomainError::TimeOverflow)?;
        Ok(Self::from_seconds(seconds))
    }

    pub fn checked_add_seconds(self, delta: i64) -> Result<Self, DomainError> {
        let seconds = self
            .seconds
            .checked_add(delta)
            .ok_or(DomainError::TimeOverflow)?;
        Ok(Self {
            seconds,
            nanos: self.nanos,
        })
    }

    /// Returns the beginning of a deterministic, epoch-aligned time bucket.
    pub fn bucket_start(self, quantum_seconds: i64) -> Result<Self, DomainError> {
        if quantum_seconds <= 0 {
            return Err(DomainError::InvalidTimeRange);
        }
        let remainder = self.seconds.rem_euclid(quantum_seconds);
        let seconds = self
            .seconds
            .checked_sub(remainder)
            .ok_or(DomainError::TimeOverflow)?;
        Ok(Self::from_seconds(seconds))
    }
}

impl Serialize for WorldInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("WorldInstant", 2)?;
        state.serialize_field("seconds", &self.seconds.to_string())?;
        state.serialize_field("nanos", &self.nanos)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for WorldInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireInstant {
            seconds: DecimalI64,
            #[serde(default)]
            nanos: u32,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DecimalI64 {
            String(String),
            Integer(i64),
        }

        let wire = WireInstant::deserialize(deserializer)?;
        let seconds = match wire.seconds {
            DecimalI64::String(value) => value.parse::<i64>().map_err(de::Error::custom)?,
            DecimalI64::Integer(value) => value,
        };
        Self::new(seconds, wire.nanos).map_err(de::Error::custom)
    }
}

impl fmt::Display for WorldInstant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.nanos == 0 {
            write!(formatter, "{}s@unix", self.seconds)
        } else {
            write!(formatter, "{}.{:09}s@unix", self.seconds, self.nanos)
        }
    }
}

/// A start-inclusive, end-exclusive validity interval. `None` means unbounded.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct TimeRange {
    pub start: WorldInstant,
    pub end: Option<WorldInstant>,
}

impl TimeRange {
    pub fn new(start: WorldInstant, end: Option<WorldInstant>) -> Result<Self, DomainError> {
        if end.is_some_and(|value| value <= start) {
            return Err(DomainError::InvalidTimeRange);
        }
        Ok(Self { start, end })
    }

    pub const fn from_start(start: WorldInstant) -> Self {
        Self { start, end: None }
    }

    pub fn validate(self) -> Result<(), DomainError> {
        if self.end.is_some_and(|value| value <= self.start) {
            return Err(DomainError::InvalidTimeRange);
        }
        Ok(())
    }

    pub fn contains(self, instant: WorldInstant) -> bool {
        instant >= self.start && self.end.is_none_or(|end| instant < end)
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireRange {
            start: WorldInstant,
            end: Option<WorldInstant>,
        }

        let wire = WireRange::deserialize(deserializer)?;
        Self::new(wire.start, wire.end).map_err(de::Error::custom)
    }
}

/// How an uncertain source date should participate in a query.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalQueryMode {
    Possible,
    Certain,
    Preferred,
}

/// An interval with independent uncertainty at both boundaries.
///
/// The possible envelope is `[begin_earliest, end_latest)`. The certain core is
/// `[begin_latest, end_earliest)` when non-empty. Original expressions and
/// probability assets are retained by the authoring record that owns this type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UncertainTimeRange {
    pub begin_earliest: WorldInstant,
    pub begin_latest: WorldInstant,
    pub end_earliest: Option<WorldInstant>,
    pub end_latest: Option<WorldInstant>,
    pub preferred: Option<TimeRange>,
}

impl<'de> Deserialize<'de> for UncertainTimeRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireUncertainRange {
            begin_earliest: WorldInstant,
            begin_latest: WorldInstant,
            end_earliest: Option<WorldInstant>,
            end_latest: Option<WorldInstant>,
            preferred: Option<TimeRange>,
        }

        let wire = WireUncertainRange::deserialize(deserializer)?;
        Self::new(
            wire.begin_earliest,
            wire.begin_latest,
            wire.end_earliest,
            wire.end_latest,
            wire.preferred,
        )
        .map_err(de::Error::custom)
    }
}

impl UncertainTimeRange {
    pub fn new(
        begin_earliest: WorldInstant,
        begin_latest: WorldInstant,
        end_earliest: Option<WorldInstant>,
        end_latest: Option<WorldInstant>,
        preferred: Option<TimeRange>,
    ) -> Result<Self, DomainError> {
        if begin_earliest > begin_latest
            || end_earliest.is_some() != end_latest.is_some()
            || matches!((end_earliest, end_latest), (Some(earliest), Some(latest)) if earliest > latest)
            || end_latest.is_some_and(|end| begin_earliest >= end)
        {
            return Err(DomainError::InvalidTemporalUncertainty);
        }
        if let Some(preferred) = preferred {
            let preferred_start_valid =
                preferred.start >= begin_earliest && preferred.start <= begin_latest;
            let preferred_end_valid = match (preferred.end, end_earliest, end_latest) {
                (None, None, None) => true,
                (Some(value), Some(earliest), Some(latest)) => value >= earliest && value <= latest,
                _ => false,
            };
            if !preferred_start_valid || !preferred_end_valid {
                return Err(DomainError::InvalidTemporalUncertainty);
            }
        }
        Ok(Self {
            begin_earliest,
            begin_latest,
            end_earliest,
            end_latest,
            preferred,
        })
    }

    pub fn range(&self, mode: TemporalQueryMode) -> Option<TimeRange> {
        match mode {
            TemporalQueryMode::Possible => Some(TimeRange {
                start: self.begin_earliest,
                end: self.end_latest,
            }),
            TemporalQueryMode::Certain => match self.end_earliest {
                Some(end) if self.begin_latest < end => Some(TimeRange {
                    start: self.begin_latest,
                    end: Some(end),
                }),
                None => Some(TimeRange::from_start(self.begin_latest)),
                Some(_) => None,
            },
            TemporalQueryMode::Preferred => self.preferred,
        }
    }
}

const fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

const fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

// Howard Hinnant's civil-date algorithm, adapted to astronomical years.
fn days_from_civil(year: i64, month: u8, day: u8) -> Option<i64> {
    let adjusted_year = year.checked_sub(i64::from(month <= 2))?;
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year.rem_euclid(400);
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_matches_calendar_constructor() {
        assert_eq!(
            WorldInstant::from_ymd_hms(1970, 1, 1, 0, 0, 0).unwrap(),
            WorldInstant::UNIX_EPOCH
        );
    }

    #[test]
    fn supports_astronomical_year_zero_and_deep_time() {
        let year_zero = WorldInstant::from_ymd_hms(0, 1, 1, 0, 0, 0).unwrap();
        let deep_time = WorldInstant::from_ymd_hms(-4_540_000_000, 1, 1, 0, 0, 0).unwrap();
        assert!(deep_time < year_zero);
        assert!(year_zero < WorldInstant::UNIX_EPOCH);
        assert_eq!(
            WorldInstant::from_ymd_hms(i64::MIN, 1, 1, 0, 0, 0),
            Err(DomainError::TimeOverflow)
        );
    }

    #[test]
    fn negative_time_buckets_use_floor_not_truncation() {
        let instant = WorldInstant::from_seconds(-1);
        assert_eq!(instant.bucket_start(60).unwrap().seconds(), -60);
        assert_eq!(
            WorldInstant::from_seconds(i64::MIN).bucket_start(60),
            Err(DomainError::TimeOverflow)
        );
    }

    #[test]
    fn ranges_are_half_open() {
        let range = TimeRange::new(
            WorldInstant::from_seconds(10),
            Some(WorldInstant::from_seconds(20)),
        )
        .unwrap();
        assert!(range.contains(WorldInstant::from_seconds(10)));
        assert!(!range.contains(WorldInstant::from_seconds(20)));
        let invalid_json = r#"{
            "start":{"seconds":"20","nanos":0},
            "end":{"seconds":"10","nanos":0}
        }"#;
        assert!(serde_json::from_str::<TimeRange>(invalid_json).is_err());
    }

    #[test]
    fn json_uses_a_string_for_large_seconds() {
        let instant = WorldInstant::from_seconds(i64::MIN + 1);
        let encoded = serde_json::to_string(&instant).unwrap();
        assert!(encoded.contains("\"seconds\":\"-9223372036854775807\""));
        assert_eq!(
            serde_json::from_str::<WorldInstant>(&encoded).unwrap(),
            instant
        );
    }

    #[test]
    fn uncertain_ranges_distinguish_possible_certain_and_preferred() {
        let uncertain = UncertainTimeRange::new(
            WorldInstant::from_seconds(10),
            WorldInstant::from_seconds(20),
            Some(WorldInstant::from_seconds(30)),
            Some(WorldInstant::from_seconds(40)),
            Some(
                TimeRange::new(
                    WorldInstant::from_seconds(15),
                    Some(WorldInstant::from_seconds(35)),
                )
                .unwrap(),
            ),
        )
        .unwrap();
        assert_eq!(
            uncertain.range(TemporalQueryMode::Possible).unwrap(),
            TimeRange::new(
                WorldInstant::from_seconds(10),
                Some(WorldInstant::from_seconds(40))
            )
            .unwrap()
        );
        assert_eq!(
            uncertain.range(TemporalQueryMode::Certain).unwrap(),
            TimeRange::new(
                WorldInstant::from_seconds(20),
                Some(WorldInstant::from_seconds(30))
            )
            .unwrap()
        );
    }
}
