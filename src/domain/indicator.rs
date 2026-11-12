#[derive(Debug, Clone, PartialEq, Eq)]
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
    ChaikinOscillator {
        fast_ma: i32,
        slow_ma: i32,
    },
    ChaikinVolatility {
        smooth_period: i32,
        chv_period: i32,
    },
    DetrendedPriceOscillator {
        period: i32,
    },
    ForceIndex {
        period: i32,
    },
    OsMa {
        fast_ema: i32,
        slow_ema: i32,
        signal_sma: i32,
    },
    Roc {
        period: i32,
    },
    Trix {
        period: i32,
    },
    Vroc {
        period: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovingAverageType {
    Ema,
    Sma,
}

#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
    pub parameters: IndicatorParameters,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> anyhow::Result<Vec<Indicator>>;
}
