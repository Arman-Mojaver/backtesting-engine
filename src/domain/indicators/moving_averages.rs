/// Simple Moving Average over a slice of `f64` values.
///
/// Returns a vector of windowed averages. The result length is
/// `values.len() - period + 1` when `values` is at least `period` long,
/// otherwise an empty vector. Pure: no side effects.
///
/// Future indicators (Bears, Bulls, TRIX, MACD, …) will add EMA, SMMA and
/// LWMA helpers to this same module.
pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }
    values
        .windows(period)
        .map(|w| w.iter().sum::<f64>() / period as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(sma(&[], 3).is_empty());
    }

    #[test]
    fn input_shorter_than_period_yields_empty() {
        assert!(sma(&[1.0, 2.0], 3).is_empty());
    }

    #[test]
    fn input_equal_to_period_yields_single_average() {
        // (1 + 2 + 3) / 3 = 2.0
        let result = sma(&[1.0, 2.0, 3.0], 3);
        assert_eq!(result, vec![2.0]);
    }

    #[test]
    fn sliding_window_averages() {
        // period 2: (1+2)/2=1.5, (2+3)/2=2.5, (3+4)/2=3.5
        let result = sma(&[1.0, 2.0, 3.0, 4.0], 2);
        assert_eq!(result, vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn period_one_returns_input_unchanged() {
        let result = sma(&[1.0, 2.0, 3.0], 1);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn period_zero_yields_empty() {
        assert!(sma(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn fractional_values_average_correctly() {
        // period 3: (0.5 + 1.5 + 1.0) / 3 = 1.0
        let result = sma(&[0.5, 1.5, 1.0], 3);
        assert_eq!(result, vec![1.0]);
    }

    #[test]
    fn long_series_window_count_is_len_minus_period_plus_one() {
        let values: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let result = sma(&values, 5);
        // 10 - 5 + 1 = 6 windows
        assert_eq!(result.len(), 6);
        // first window (0+1+2+3+4)/5 = 2.0
        assert_eq!(result[0], 2.0);
        // last window (5+6+7+8+9)/5 = 7.0
        assert_eq!(result[5], 7.0);
    }
}
