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
        return Ok($crate::domain::signal::OscillatorBuffer { items: Vec::new() })
    };
}

#[macro_export]
macro_rules! empty_crossover {
    () => {
        return Ok($crate::domain::signal::CrossoverBuffers {
            upcross_long_buffer: Vec::new(),
            upcross_short_buffer: Vec::new(),
        })
    };
}
