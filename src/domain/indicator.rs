/// The configurable setup of one indicator: which indicator to compute
/// and the parameters it is computed with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndicatorParameters {
    /// Moving Average Convergence Divergence: the MACD line
    /// `MA(close, fast) - MA(close, slow)` crossed against its signal line
    /// `SMA(MACD, 9)` (the signal period is an engine constant).
    Macd {
        /// Moving-average type of the slow line ("ema" or "sma").
        slow_type: String,
        /// Period of the slow line, in bars.
        slow_n: i32,
        /// Moving-average type of the fast line ("ema" or "sma").
        fast_type: String,
        /// Period of the fast line, in bars.
        fast_n: i32,
    },
    /// Relative Strength Index of the close, compared against explicit
    /// overbought and oversold levels for its signals.
    Rsi {
        /// Lookback of price differences, in bars.
        n: i32,
        /// Overbought level, e.g. 70.
        high_threshold: i32,
        /// Oversold level, e.g. 30.
        low_threshold: i32,
    },
    /// Awesome Oscillator: the 5-bar minus the 34-bar simple moving
    /// average of the median price. No configurable parameters.
    AwesomeOscillator,
    /// Accelerator/Decelerator oscillator: the Awesome Oscillator minus
    /// its own 5-bar moving average. No configurable parameters.
    AcceleratorDecelerator,
    /// Bears Power: how far the low sits below the exponential moving
    /// average of the closes.
    BearsPower {
        /// Period of the EMA of closes, in bars.
        period: i32,
    },
    /// Bulls Power: how far the high sits above the exponential moving
    /// average of the closes.
    BullsPower {
        /// Period of the EMA of closes, in bars.
        period: i32,
    },
    /// Chaikin Oscillator: the fast EMA minus the slow EMA of the
    /// accumulation/distribution line.
    ChaikinOscillator {
        /// Period of the fast EMA, in bars.
        fast_ma: i32,
        /// Period of the slow EMA, in bars.
        slow_ma: i32,
    },
    /// Chaikin Volatility: the rate of change of an EMA-smoothed
    /// high-low spread.
    ChaikinVolatility {
        /// Period of the EMA smoothing the high-low spread, in bars.
        smooth_period: i32,
        /// Lookback of the rate of change, in bars.
        chv_period: i32,
    },
    /// Detrended Price Oscillator: the close minus a shifted moving
    /// average, removing the trend.
    DetrendedPriceOscillator {
        /// Nominal period, in bars; the underlying SMA uses period / 2 + 1 bars.
        period: i32,
    },
    /// Force Index: volume weighted by the bar-to-bar change of a
    /// moving average of the closes.
    ForceIndex {
        /// Period of the SMA of closes, in bars.
        period: i32,
    },
    /// OsMA, the MACD histogram: the MACD line minus its signal line.
    OsMa {
        /// Period of the fast EMA of closes, in bars.
        fast_ema: i32,
        /// Period of the slow EMA of closes, in bars.
        slow_ema: i32,
        /// Period of the SMA signal line over the MACD, in bars.
        signal_sma: i32,
    },
    /// Rate of Change: the percentage change of the close over the period.
    Roc {
        /// Lookback of the price change, in bars.
        period: i32,
    },
    /// TRIX: the one-bar rate of change of a triple-smoothed EMA of closes.
    Trix {
        /// Period of each of the three EMA smoothings, in bars.
        period: i32,
    },
    /// Volume Rate of Change: ROC applied to volume instead of price.
    Vroc {
        /// Lookback of the volume change, in bars.
        period: i32,
    },
}

#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
    pub parameters: IndicatorParameters,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> anyhow::Result<Vec<Indicator>>;
}
