use crate::domain::cast;
/// Simple Moving Average over a slice of `f64` values.
///
/// Returns a vector of windowed averages. The result length is
/// `values.len() - period + 1` when `values` is at least `period` long,
/// otherwise an empty vector. Pure: no side effects.
///
/// Only SMA and EMA are needed by the current indicator set.
pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }
    values
        .windows(period)
        .map(|w| w.iter().sum::<f64>() / cast::usize_to_f64(period))
        .collect()
}

/// Exponential Moving Average over a slice of `f64` values.
///
/// Seeds with the SMA of the first `period` values (the standard MQL5 EMA
/// definition), then recurses with smoothing factor `k = 2 / (period + 1)`:
///
/// ```text
/// ema[0]      = SMA(values[0..period])            // index period-1
/// ema[j]      = (values[period-1+j] - ema[j-1]) * k + ema[j-1]
/// ```
///
/// The result length is `values.len() - period + 1` when `values` is at least
/// `period` long, otherwise an empty vector. Entry `j` corresponds to the
/// original series index `period - 1 + j`. Pure: no side effects.
pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }
    let k = 2.0 / (cast::usize_to_f64(period) + 1.0);
    let mut out = Vec::with_capacity(values.len().saturating_sub(period.saturating_sub(1)));
    // Seed: SMA of the first `period` values (index period-1 in the series).
    let seed = values.iter().take(period).sum::<f64>() / cast::usize_to_f64(period);
    out.push(seed);
    // Recurse: ema[j] from ema[j-1] and values[period-1+j] (j = 1..).
    let mut prev = seed;
    for &current in values.iter().skip(period) {
        let next = (current - prev).mul_add(k, prev);
        out.push(next);
        prev = next;
    }
    out
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
        let values: Vec<f64> = (0..10).map(|i: i32| f64::from(i)).collect();
        let result = sma(&values, 5);
        // 10 - 5 + 1 = 6 windows
        assert_eq!(result.len(), 6);
        // first window (0+1+2+3+4)/5 = 2.0
        assert_eq!(result[0], 2.0);
        // last window (5+6+7+8+9)/5 = 7.0
        assert_eq!(result[5], 7.0);
    }

    // --- EMA unit tests ---

    #[test]
    fn ema_empty_input_yields_empty_output() {
        assert!(ema(&[], 3).is_empty());
    }

    #[test]
    fn ema_input_shorter_than_period_yields_empty() {
        assert!(ema(&[1.0, 2.0], 3).is_empty());
    }

    #[test]
    fn ema_input_equal_to_period_yields_single_sma_seed() {
        // (1 + 2 + 3) / 3 = 2.0 — only the seed, no recursion.
        let result = ema(&[1.0, 2.0, 3.0], 3);
        assert_eq!(result, vec![2.0]);
    }

    #[test]
    fn ema_period_zero_yields_empty() {
        assert!(ema(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn ema_constant_series_yields_constant_ema() {
        // Constant 10.0 -> seed 10.0, recursion stays 10.0.
        let values = vec![10.0; 6];
        let result = ema(&values, 3);
        assert_eq!(result, vec![10.0, 10.0, 10.0, 10.0]);
    }

    #[test]
    fn ema_linear_ramp_matches_hand_computed_recursion() {
        // values = [0,1,2,3,4], period 3 -> k = 2/(3+1) = 0.5
        // seed      = SMA(0,1,2) = 1.0            (index 2)
        // ema[1]    = (3 - 1.0)*0.5 + 1.0 = 2.0   (index 3)
        // ema[2]    = (4 - 2.0)*0.5 + 2.0 = 3.0   (index 4)
        let result = ema(&[0.0, 1.0, 2.0, 3.0, 4.0], 3);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn ema_output_length_is_len_minus_period_plus_one() {
        let values: Vec<f64> = (0..10).map(|i: i32| f64::from(i)).collect();
        let result = ema(&values, 4);
        // 10 - 4 + 1 = 7
        assert_eq!(result.len(), 7);
    }
}
