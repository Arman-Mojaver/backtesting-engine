/// Access the `items` slice of an oscillator or threshold buffer.
#[macro_export]
macro_rules! indicator_values {
    ($buffer:expr) => {
        &$buffer.items
    };
}
