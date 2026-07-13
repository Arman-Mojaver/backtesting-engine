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
    /// Chaikin Volatility — `100 * (SHL[i] - SHL[i - chv_period]) /
    /// SHL[i - chv_period])` where `SHL = EMA(high - low, smooth_period)`.
    /// Two configurable periods (defaults 10 and 10 per the MQL5 `CHV.mq5`
    /// source). The MA method is baked in as `EMA` per umbrella PRD
    /// Decision 8.
    ChaikinVolatility { smooth_period: i32, chv_period: i32 },
    /// Detrended Price Oscillator — `close[i] - SMA(close, ExtMAPeriod)[i]`
    /// where `ExtMAPeriod = InpDetrendPeriod / 2 + 1` (integer division). One
    /// configurable period (default 12 per the MQL5 `DPO.mq5` source). The MA
    /// method is baked in as `SMA` per umbrella PRD Decision 8.
    DetrendedPriceOscillator { period: i32 },
    /// Force Index — `volume[i] * (SMA(close, period)[i] -
    /// SMA(close, period)[i - 1])` with a configurable SMA period (default 13
    /// per the MQL5 `Force_Index.mq5` source). The MA method is baked in as
    /// `MODE_SMA`; the applied price is `PRICE_CLOSE`; the volume type is
    /// `VOLUME_TICK` (using `point.volume` as f64, per umbrella PRD Decision 7).
    ForceIndex { period: i32 },
    /// OsMA (Moving Average of Oscillator, aka MACD histogram) —
    /// `MACD - Signal` where `MACD = EMA(close, fast_ema) - EMA(close,
    /// slow_ema)` and `Signal = SMA(MACD, signal_sma)`. Three configurable
    /// periods (defaults 12, 26, 9 per the MQL5 `OsMA.mq5` source). The MA
    /// methods are baked in as `MODE_EMA` for fast/slow and `MODE_SMA` for
    /// the signal line; the applied price is `PRICE_CLOSE` (per umbrella PRD
    /// Decision 8).
    OsMa {
        fast_ema: i32,
        slow_ema: i32,
        signal_sma: i32,
    },
    /// Rate of Change — `(close[i] - close[i - period]) / close[i] * 100`
    /// with a configurable period (default 12 per the MQL5 `ROC.mq5` source).
    /// The applied price is baked in as `PRICE_CLOSE` (per umbrella PRD
    /// Decision 8). No MA helpers are needed — ROC is a pure price-momentum
    /// formula.
    Roc { period: i32 },
    /// TRIX (Triple Exponential Average) — the rate of change of a
    /// triple-smoothed EMA of the close price:
    /// `TRIX[k] = (EMA3[k + 1] - EMA3[k]) / EMA3[k]` where
    /// `EMA3 = EMA(EMA(EMA(close, period)))`. One configurable period
    /// (default 14 per the MQL5 `TRIX.mq5` source). The applied price is
    /// baked in as `PRICE_CLOSE` (per umbrella PRD Decision 8). The `ema`
    /// helper from `moving_averages.rs` is reused three times.
    Trix { period: i32 },
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
