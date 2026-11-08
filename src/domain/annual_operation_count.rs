use crate::domain::cast;

const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

pub fn difference_in_years(start_timestamp: i64, end_timestamp: i64) -> f64 {
    let seconds = end_timestamp.saturating_sub(start_timestamp);
    if seconds <= 0 {
        return 0.0;
    }
    cast::i64_to_f64(seconds) / SECONDS_PER_YEAR
}

pub fn annual_operation_count(total_count: usize, start_timestamp: i64, end_timestamp: i64) -> f64 {
    let years = difference_in_years(start_timestamp, end_timestamp);
    if years <= 0.0 {
        return 0.0;
    }

    let annualized = cast::usize_to_f64(total_count) / years;
    cast::round_to(annualized, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::fixtures::ONE_YEAR_SECS;

    const TWO_YEARS_SECS: i64 = 2 * ONE_YEAR_SECS;

    #[test]
    fn difference_in_years_returns_one_for_julian_year() {
        assert_eq!(difference_in_years(0, ONE_YEAR_SECS), 1.0);
    }

    #[test]
    fn difference_in_years_returns_half_for_half_julian_year() {
        assert_eq!(difference_in_years(0, ONE_YEAR_SECS / 2), 0.5);
    }

    #[test]
    fn difference_in_years_is_positive_for_one_second_span() {
        assert!(difference_in_years(0, 1) > 0.0);
    }

    #[test]
    fn difference_in_years_returns_zero_for_zero_span() {
        assert_eq!(difference_in_years(1000, 1000), 0.0);
    }

    #[test]
    fn difference_in_years_returns_zero_for_negative_span() {
        assert_eq!(difference_in_years(2000, 1000), 0.0);
    }

    #[test]
    fn difference_in_years_saturates_on_extreme_timestamps() {
        let years = difference_in_years(i64::MIN, i64::MAX);
        assert!(years.is_finite() && years > 0.0);
    }

    #[test]
    fn zero_count_returns_zero() {
        let result = annual_operation_count(0, 0, ONE_YEAR_SECS);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn zero_range_returns_zero() {
        let result = annual_operation_count(10, 1000, 1000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn negative_range_returns_zero() {
        let result = annual_operation_count(10, 2000, 1000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn zero_ops_zero_range_returns_zero() {
        let result = annual_operation_count(0, 0, 1000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn one_year_count_1() {
        let result = annual_operation_count(1, 0, ONE_YEAR_SECS);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn one_year_count_2() {
        let result = annual_operation_count(2, 0, ONE_YEAR_SECS);
        assert_eq!(result, 2.0);
    }

    #[test]
    fn one_year_count_3() {
        let result = annual_operation_count(3, 0, ONE_YEAR_SECS);
        assert_eq!(result, 3.0);
    }

    #[test]
    fn one_year_count_4() {
        let result = annual_operation_count(4, 0, ONE_YEAR_SECS);
        assert_eq!(result, 4.0);
    }

    #[test]
    fn one_year_count_100() {
        let result = annual_operation_count(100, 0, ONE_YEAR_SECS);
        assert_eq!(result, 100.0);
    }

    #[test]
    fn two_years_count_1() {
        let result = annual_operation_count(1, 0, TWO_YEARS_SECS);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn two_years_count_2() {
        let result = annual_operation_count(2, 0, TWO_YEARS_SECS);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn two_years_count_3() {
        let result = annual_operation_count(3, 0, TWO_YEARS_SECS);
        assert_eq!(result, 1.5);
    }

    #[test]
    fn two_years_count_4() {
        let result = annual_operation_count(4, 0, TWO_YEARS_SECS);
        assert_eq!(result, 2.0);
    }

    #[test]
    fn two_years_count_5() {
        let result = annual_operation_count(5, 0, TWO_YEARS_SECS);
        assert_eq!(result, 2.5);
    }

    #[test]
    fn two_years_count_300() {
        let result = annual_operation_count(300, 0, TWO_YEARS_SECS);
        assert_eq!(result, 150.0);
    }

    #[test]
    fn one_op_per_day_over_one_year_returns_365() {
        let result = annual_operation_count(365, 0, ONE_YEAR_SECS);
        assert_eq!(result, 365.0);
    }

    #[test]
    fn offset_start_produces_same_result_as_zero_start() {
        let from_zero = annual_operation_count(10, 0, ONE_YEAR_SECS);
        let from_offset = annual_operation_count(10, 1_000_000, 1_000_000 + ONE_YEAR_SECS);
        assert_eq!(from_zero, from_offset);
    }

    #[test]
    fn one_op_over_ten_years_returns_one_tenth() {
        let ten_years_secs: i64 = 10 * ONE_YEAR_SECS;
        let result = annual_operation_count(1, 0, ten_years_secs);
        assert_eq!(result, 0.10);
    }
}
