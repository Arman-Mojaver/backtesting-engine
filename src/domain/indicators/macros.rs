/// Access the `items` slice of an oscillator or threshold buffer.
#[macro_export]
macro_rules! indicator_values {
    ($buffer:expr) => {
        &$buffer.items
    };
}

/// Validate one or more period parameters, returning the appropriate error
/// variant for the first non-positive value.
///
/// Each pair is `(param, ErrorVariant)`. Separate multiple pairs with `;`.
///
/// # Examples
/// ```ignore
/// validate_period!(period, InvalidBearsPeriod);
/// validate_period!(fast_ma, InvalidChoFastPeriod; slow_ma, InvalidChoSlowPeriod);
/// ```
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
