use crate::domain::cast;
use crate::domain::time_span::difference_in_years;

pub fn annual_operation_count(total_count: usize, start_timestamp: i64, end_timestamp: i64) -> f64 {
    let Some(years) = difference_in_years(start_timestamp, end_timestamp) else {
        return 0.0;
    };

    let annualized = cast::usize_to_f64(total_count) / years;
    cast::round_to(annualized, cast::METRIC_DECIMALS)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::fixtures::ONE_YEAR_SECS;

    const TWO_YEARS_SECS: i64 = 2 * ONE_YEAR_SECS;

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
        let result = annual_operation_count(0, 1000, 1000);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn one_year_span_returns_total_count() {
        for count in [1, 2, 3, 4, 100, 365] {
            let result = annual_operation_count(count, 0, ONE_YEAR_SECS);
            assert_eq!(result, cast::usize_to_f64(count), "count = {count}");
        }
    }

    #[test]
    fn two_year_span_halves_total_count() {
        for count in [1, 2, 3, 4, 5, 300] {
            let result = annual_operation_count(count, 0, TWO_YEARS_SECS);
            assert_eq!(result, cast::usize_to_f64(count) / 2.0, "count = {count}");
        }
    }

    #[test]
    fn result_is_rounded_to_two_decimals() {
        let three_years_secs: i64 = 3 * ONE_YEAR_SECS;
        assert_eq!(annual_operation_count(1, 0, three_years_secs), 0.33);
        assert_eq!(annual_operation_count(2, 0, three_years_secs), 0.67);
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
