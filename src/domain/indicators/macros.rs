#[macro_export]
macro_rules! indicator_values {
    ($buffer:expr) => {
        &$buffer.items
    };
}

#[macro_export]
macro_rules! validate_period {
    ($($param:expr, $error_variant:ident);+ $(;)?) => {
        $(
            if $param <= 0 {
                return Err($crate::domain::indicators::IndicatorError::$error_variant { n: $param });
            }
        )+
    };
}

#[macro_export]
macro_rules! empty_oscillator {
    () => {
        return Ok($crate::domain::signal::OscillatorSeries { items: Vec::new() })
    };
}
