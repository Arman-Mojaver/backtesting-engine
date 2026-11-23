use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorParameters {
    Macd {
        slow_type: MovingAverageType,
        slow_n: i32,
        fast_type: MovingAverageType,
        fast_n: i32,
    },
    Rsi {
        n: i32,
        /// RSI levels on its 0–100 scale, e.g. 70 overbought and 30 oversold.
        high_threshold: i32,
        low_threshold: i32,
    },
    AwesomeOscillator,
    AcceleratorDecelerator,
    BearsPower {
        period: i32,
    },
    BullsPower {
        period: i32,
    },
    Cci {
        period: i32,
    },
    ChaikinOscillator {
        fast_ma: i32,
        slow_ma: i32,
    },
    ChaikinVolatility {
        smooth_period: i32,
        chv_period: i32,
    },
    Dema {
        period: i32,
    },
    DetrendedPriceOscillator {
        period: i32,
    },
    ForceIndex {
        period: i32,
    },
    Mfi {
        period: i32,
        /// MFI levels on its 0–100 scale, e.g. 80 overbought and 20 oversold.
        high_threshold: i32,
        low_threshold: i32,
    },
    OsMa {
        fast_ema: i32,
        slow_ema: i32,
        signal_sma: i32,
    },
    Roc {
        period: i32,
    },
    Rvi {
        period: i32,
    },
    Stochastic {
        k_period: i32,
        slowing: i32,
        /// %K levels on its 0–100 scale, e.g. 80 overbought and 20 oversold.
        high_threshold: i32,
        low_threshold: i32,
    },
    Tema {
        period: i32,
    },
    Trix {
        period: i32,
    },
    Vroc {
        period: i32,
    },
    Wpr {
        period: i32,
        /// %R levels on its -100–0 scale, e.g. -20 overbought and -80 oversold.
        high_threshold: i32,
        low_threshold: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovingAverageType {
    Ema,
    Sma,
}

#[derive(Debug, Clone, Copy)]
pub struct Indicator {
    pub id: i32,
    pub parameters: IndicatorParameters,
}

pub fn unprocessed_indicators(
    indicators: &[Indicator],
    processed_ids: &FxHashSet<i32>,
) -> Vec<Indicator> {
    indicators
        .iter()
        .filter(|indicator| !processed_ids.contains(&indicator.id))
        .copied()
        .collect()
}

pub fn indicator_ids(indicators: &[Indicator]) -> Vec<i32> {
    indicators.iter().map(|indicator| indicator.id).collect()
}

pub trait IndicatorRepository {
    fn get_all(&mut self) -> anyhow::Result<Vec<Indicator>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indicators(ids: &[i32]) -> Vec<Indicator> {
        ids.iter()
            .map(|&id| Indicator {
                id,
                parameters: IndicatorParameters::AwesomeOscillator,
            })
            .collect()
    }

    #[test]
    fn unprocessed_indicators_with_no_indicators_returns_empty() {
        let result = unprocessed_indicators(&[], &FxHashSet::default());
        assert!(result.is_empty());
    }

    #[test]
    fn unprocessed_indicators_with_nothing_processed_returns_all() {
        let result = unprocessed_indicators(&indicators(&[1, 2, 3]), &FxHashSet::default());
        assert_eq!(indicator_ids(&result), vec![1, 2, 3]);
    }

    #[test]
    fn unprocessed_indicators_with_all_processed_returns_empty() {
        let mut processed = FxHashSet::default();
        processed.insert(1);
        processed.insert(2);
        let result = unprocessed_indicators(&indicators(&[1, 2]), &processed);
        assert!(result.is_empty());
    }

    #[test]
    fn unprocessed_indicators_with_partial_overlap_returns_the_rest() {
        let mut processed = FxHashSet::default();
        processed.insert(1);
        processed.insert(3);
        let result = unprocessed_indicators(&indicators(&[1, 2, 3, 4]), &processed);
        assert_eq!(indicator_ids(&result), vec![2, 4]);
    }

    #[test]
    fn unprocessed_indicators_with_unrelated_processed_ids_returns_all() {
        let mut processed = FxHashSet::default();
        processed.insert(10);
        let result = unprocessed_indicators(&indicators(&[1, 2]), &processed);
        assert_eq!(indicator_ids(&result), vec![1, 2]);
    }
}
