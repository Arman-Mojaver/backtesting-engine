use crate::domain::cast;

const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

pub fn difference_in_years(start_timestamp: i64, end_timestamp: i64) -> Option<f64> {
    let seconds = end_timestamp.saturating_sub(start_timestamp);
    if seconds <= 0 {
        return None;
    }
    Some(cast::i64_to_f64(seconds) / SECONDS_PER_YEAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::fixtures::ONE_YEAR_SECS;

    #[test]
    fn difference_in_years_returns_one_for_julian_year() {
        assert_eq!(difference_in_years(0, ONE_YEAR_SECS), Some(1.0));
    }

    #[test]
    fn difference_in_years_returns_half_for_half_julian_year() {
        assert_eq!(difference_in_years(0, ONE_YEAR_SECS / 2), Some(0.5));
    }

    #[test]
    fn difference_in_years_is_positive_for_one_second_span() {
        assert!(difference_in_years(0, 1).is_some_and(|years| years > 0.0));
    }

    #[test]
    fn difference_in_years_returns_none_for_zero_span() {
        assert_eq!(difference_in_years(1000, 1000), None);
    }

    #[test]
    fn difference_in_years_returns_none_for_negative_span() {
        assert_eq!(difference_in_years(2000, 1000), None);
    }

    #[test]
    fn difference_in_years_saturates_on_extreme_timestamps() {
        let years = difference_in_years(i64::MIN, i64::MAX);
        assert!(years.is_some_and(|years| years.is_finite() && years > 0.0));
    }
}
