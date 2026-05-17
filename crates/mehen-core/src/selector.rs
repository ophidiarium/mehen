use core::fmt;
use core::str::FromStr;

use crate::MetricKey;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A metric reference used by `mehen diff --threshold`, `mehen top-offenders
/// --metric`, and the action's `metrics` input.
///
/// Format examples:
///
/// - `cognitive` — bare key; maps to [`SelectorAggregator::Root`] (file-
///   level / root-unit value only, no aggregation across nested spaces).
/// - `cognitive.max` — explicit max-of-spaces aggregator.
/// - `loc.lloc` — namespaced metric, also resolves to
///   [`SelectorAggregator::Root`].
/// - `loc.lloc.sum` — namespaced metric with explicit aggregator.
///
/// Aggregator suffixes recognized: `min`, `max`, `avg`, `sum`. Anything else
/// is treated as part of the metric key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricSelector {
    pub key: MetricKey,
    pub aggregator: SelectorAggregator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorAggregator {
    /// Compute on the file-level (root unit) value only.
    Root,
    Min,
    Max,
    Avg,
    Sum,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorParseError(String);

impl fmt::Display for SelectorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid metric selector: `{}`", self.0)
    }
}

impl core::error::Error for SelectorParseError {}

impl FromStr for MetricSelector {
    type Err = SelectorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(SelectorParseError(s.to_string()));
        }
        let (key_part, aggregator) = match trimmed.rsplit_once('.') {
            Some((rest, suffix)) => match suffix {
                "min" => (rest, SelectorAggregator::Min),
                "max" => (rest, SelectorAggregator::Max),
                "avg" => (rest, SelectorAggregator::Avg),
                "sum" => (rest, SelectorAggregator::Sum),
                _ => (trimmed, SelectorAggregator::Root),
            },
            None => (trimmed, SelectorAggregator::Root),
        };

        if key_part.is_empty() {
            return Err(SelectorParseError(s.to_string()));
        }

        Ok(Self {
            key: MetricKey::new(key_part.to_string()),
            aggregator,
        })
    }
}

impl fmt::Display for MetricSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.aggregator {
            SelectorAggregator::Root => write!(f, "{}", self.key),
            SelectorAggregator::Min => write!(f, "{}.min", self.key),
            SelectorAggregator::Max => write!(f, "{}.max", self.key),
            SelectorAggregator::Avg => write!(f, "{}.avg", self.key),
            SelectorAggregator::Sum => write!(f, "{}.sum", self.key),
        }
    }
}

impl Serialize for MetricSelector {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MetricSelector {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_metric() {
        let s: MetricSelector = "cognitive".parse().unwrap();
        assert_eq!(s.key.as_str(), "cognitive");
        assert_eq!(s.aggregator, SelectorAggregator::Root);
    }

    #[test]
    fn parses_namespaced_metric() {
        let s: MetricSelector = "loc.lloc".parse().unwrap();
        assert_eq!(s.key.as_str(), "loc.lloc");
        assert_eq!(s.aggregator, SelectorAggregator::Root);
    }

    #[test]
    fn parses_aggregator_suffix() {
        let s: MetricSelector = "cyclomatic.max".parse().unwrap();
        assert_eq!(s.key.as_str(), "cyclomatic");
        assert_eq!(s.aggregator, SelectorAggregator::Max);
    }

    #[test]
    fn parses_namespaced_with_aggregator() {
        let s: MetricSelector = "loc.lloc.sum".parse().unwrap();
        assert_eq!(s.key.as_str(), "loc.lloc");
        assert_eq!(s.aggregator, SelectorAggregator::Sum);
    }

    #[test]
    fn rejects_empty() {
        assert!("".parse::<MetricSelector>().is_err());
        assert!(".max".parse::<MetricSelector>().is_err());
    }

    #[test]
    fn round_trip_via_display() {
        for input in [
            "cognitive",
            "loc.lloc",
            "cyclomatic.max",
            "halstead.volume.avg",
        ] {
            let parsed: MetricSelector = input.parse().unwrap();
            assert_eq!(parsed.to_string(), input);
        }
    }
}
