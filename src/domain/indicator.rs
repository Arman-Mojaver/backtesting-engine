#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorParameters {
    Macd {
        slow_type: String,
        slow_n: i32,
        fast_type: String,
        fast_n: i32,
    },
    Rsi {
        n: i32,
        high_threshold: i32,
        low_threshold: i32,
    },
    /// Awesome Oscillator — fixed 5/34 SMA of the median price, no
    /// configurable parameters (per the MQL5 source).
    AwesomeOscillator,
    /// Accelerator/Decelerator — fixed 5/34 SMA of the median price with
    /// AO minus SMA(AO,5), no configurable parameters (per the MQL5
    /// `Accelerator.mq5` source).
    AcceleratorDecelerator,
    /// Bears Power — `low[i] - EMA(close, period)[i]` with a configurable
    /// EMA period (default 13 per the MQL5 `Bears.mq5` source). The MA method
    /// is baked in as `MODE_EMA` per umbrella PRD Decision 8.
    BearsPower { period: i32 },
    /// Bulls Power — `high[i] - EMA(close, period)[i]` with a configurable
    /// EMA period (default 13 per the MQL5 `Bulls.mq5` source). The MA method
    /// is baked in as `MODE_EMA` per umbrella PRD Decision 8.
    BullsPower { period: i32 },
    /// Chaikin Oscillator — `EMA(AD, fast_ma) - EMA(AD, slow_ma)` where AD
    /// is the Accumulation/Distribution line (inlined). Two configurable MA
    /// periods (defaults 3 and 10 per the MQL5 `CHO.mq5` source). The MA
    /// method is baked in as `MODE_EMA`; the volume type is `VOLUME_TICK`
    /// (using `point.volume` as f64, per umbrella PRD Decision 5).
    ChaikinOscillator { fast_ma: i32, slow_ma: i32 },
}

#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
    pub r#type: String,
    pub parameters: IndicatorParameters,
    pub identifier: String,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> Vec<Indicator>;
    fn get_by_type(&self, indicator_type: &str) -> Vec<Indicator>;
}
