const SECONDS_PER_DAY: f64 = 86400.0;
const DAYS_PER_YEAR: f64 = 365.25;

pub fn annual_operation_count(total_count: usize, start_timestamp: i64, end_timestamp: i64) -> f64 {
    if total_count == 0 {
        return 0.0;
    }

    let days = (end_timestamp - start_timestamp) as f64 / SECONDS_PER_DAY;
    if days <= 0.0 {
        return 0.0;
    }

    let annualized = (total_count as f64 / days) * DAYS_PER_YEAR;
    (annualized * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_YEAR_SECS: i64 = 365 * 86400;
    const TWO_YEARS_SECS: i64 = 730 * 86400;

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
        assert!((result - 1.0).abs() < 0.01);
    }

    #[test]
    fn one_year_count_2() {
        let result = annual_operation_count(2, 0, ONE_YEAR_SECS);
        assert!((result - 2.0).abs() < 0.01);
    }

    #[test]
    fn one_year_count_3() {
        let result = annual_operation_count(3, 0, ONE_YEAR_SECS);
        assert!((result - 3.0).abs() < 0.02);
    }

    #[test]
    fn one_year_count_4() {
        let result = annual_operation_count(4, 0, ONE_YEAR_SECS);
        assert!((result - 4.0).abs() < 0.02);
    }

    #[test]
    fn one_year_count_100() {
        let result = annual_operation_count(100, 0, ONE_YEAR_SECS);
        assert!((result - 100.0).abs() < 0.1);
    }

    #[test]
    fn two_years_count_1() {
        let result = annual_operation_count(1, 0, TWO_YEARS_SECS);
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn two_years_count_2() {
        let result = annual_operation_count(2, 0, TWO_YEARS_SECS);
        assert!((result - 1.0).abs() < 0.01);
    }

    #[test]
    fn two_years_count_3() {
        let result = annual_operation_count(3, 0, TWO_YEARS_SECS);
        assert!((result - 1.5).abs() < 0.01);
    }

    #[test]
    fn two_years_count_4() {
        let result = annual_operation_count(4, 0, TWO_YEARS_SECS);
        assert!((result - 2.0).abs() < 0.01);
    }

    #[test]
    fn two_years_count_5() {
        let result = annual_operation_count(5, 0, TWO_YEARS_SECS);
        assert!((result - 2.5).abs() < 0.01);
    }

    #[test]
    fn two_years_count_300() {
        let result = annual_operation_count(300, 0, TWO_YEARS_SECS);
        assert!((result - 150.0).abs() < 0.2);
    }

    #[test]
    fn rounding_rounds_up_at_point_five_boundary() {
        let result = annual_operation_count(8, 0, ONE_YEAR_SECS);
        assert_eq!(result, 8.01);
    }

    #[test]
    fn offset_start_produces_same_result_as_zero_start() {
        let from_zero = annual_operation_count(10, 0, ONE_YEAR_SECS);
        let from_offset = annual_operation_count(10, 1_000_000, 1_000_000 + ONE_YEAR_SECS);
        assert_eq!(from_zero, from_offset);
    }

    #[test]
    fn exact_value_when_count_equals_days() {
        let result = annual_operation_count(365, 0, ONE_YEAR_SECS);
        assert_eq!(result, 365.25);
    }

    #[test]
    fn near_zero_rounds_to_small_nonzero() {
        let ten_years_secs: i64 = 3650 * 86400;
        let result = annual_operation_count(1, 0, ten_years_secs);
        assert_eq!(result, 0.10);
    }
}
