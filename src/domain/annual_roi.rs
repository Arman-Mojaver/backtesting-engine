const SECONDS_PER_DAY: f64 = 86400.0;
const DAYS_PER_YEAR: f64 = 365.25;

fn difference_in_years(start_timestamp: i64, end_timestamp: i64) -> f64 {
    let days = (end_timestamp - start_timestamp) as f64 / SECONDS_PER_DAY;
    if days <= 0.0 {
        return 0.0;
    }
    (days / DAYS_PER_YEAR * 100.0).round() / 100.0
}

pub fn annual_roi(start_timestamp: i64, end_timestamp: i64, global_roi: f64) -> f64 {
    let years = difference_in_years(start_timestamp, end_timestamp);
    if years <= 0.0 {
        return 0.0;
    }

    let global_accumulated = 1.0 + global_roi / 100.0;
    let annual_accumulated = global_accumulated.powf(1.0 / years);
    ((annual_accumulated - 1.0) * 10000.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_YEAR_SECS: i64 = 365 * 86400;
    const HALF_YEAR_SECS: i64 = 182 * 86400;
    const QUARTER_YEAR_SECS: i64 = 91 * 86400;
    const THIRD_YEAR_SECS: i64 = 122 * 86400;

    #[test]
    fn zero_span_returns_zero() {
        assert_eq!(annual_roi(1000, 1000, 10.0), 0.0);
    }

    #[test]
    fn negative_span_returns_zero() {
        assert_eq!(annual_roi(2000, 1000, 10.0), 0.0);
    }

    #[test]
    fn one_year_compounds_to_same_global_roi() {
        // 1 year: annual = global
        assert_eq!(annual_roi(0, ONE_YEAR_SECS, 10.0), 10.0);
    }

    #[test]
    fn two_years_compounds_to_global() {
        // 2 years, global 21%: annual = ((1.21)^(1/2) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, 2 * ONE_YEAR_SECS, 21.0), 10.0);
    }

    #[test]
    fn three_years_compounds_to_global() {
        // 3 years, global 33.1%: annual = ((1.331)^(1/3) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, 3 * ONE_YEAR_SECS, 33.1), 10.0);
    }

    #[test]
    fn four_years_compounds_to_global() {
        // 4 years, global 46.41%: annual = ((1.4641)^(1/4) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, 4 * ONE_YEAR_SECS, 46.41), 10.0);
    }

    #[test]
    fn five_years_compounds_to_global() {
        // 5 years, global 61.05%: annual = ((1.6105)^(1/5) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, 5 * ONE_YEAR_SECS, 61.05), 10.0);
    }

    #[test]
    fn half_year_compounds_to_global() {
        // 0.5 years, global 4.88%: annual = ((1.0488)^(1/0.5) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, HALF_YEAR_SECS, 4.88), 10.0);
    }

    #[test]
    fn one_and_half_years_compounds_to_global() {
        // 1.5 years, global 15.37%: annual = ((1.1537)^(1/1.5) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, ONE_YEAR_SECS + HALF_YEAR_SECS, 15.37), 10.0);
    }

    #[test]
    fn quarter_year_compounds_to_global() {
        // 0.25 years, global 2.411%: annual = ((1.02411)^(1/0.25) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, QUARTER_YEAR_SECS, 2.411), 10.0);
    }

    #[test]
    fn third_of_year_compounds_to_global() {
        // 0.33 years, global 3.195%: annual = ((1.03195)^(1/0.33) - 1) * 100 = 10.0
        assert_eq!(annual_roi(0, THIRD_YEAR_SECS, 3.195), 10.0);
    }

    #[test]
    fn offset_start_produces_same_result_as_zero_start() {
        let from_zero = annual_roi(0, ONE_YEAR_SECS, 10.0);
        let from_offset = annual_roi(1_000_000, 1_000_000 + ONE_YEAR_SECS, 10.0);
        assert_eq!(from_zero, from_offset);
    }

    #[test]
    fn negative_global_roi_annualises() {
        // 1 year, global -10%: annual = -10.0
        assert_eq!(annual_roi(0, ONE_YEAR_SECS, -10.0), -10.0);
    }

    #[test]
    fn zero_global_roi_returns_zero() {
        assert_eq!(annual_roi(0, ONE_YEAR_SECS, 0.0), 0.0);
    }
}
