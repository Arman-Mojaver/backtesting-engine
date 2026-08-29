//! Declarative macros to eliminate repetitive indicator test boilerplate.

/// Access the `items` slice of an oscillator or threshold buffer.
///
/// Replaces the per-module `fn values(buffer: &X) -> &[IndicatorValue]` helper
/// that was duplicated across every indicator test module.
///
/// # Example
/// ```ignore
/// let result = compute(&points, 13).unwrap();
/// let vals = indicator_values!(result);
/// assert_eq!(vals.len(), 2387);
/// ```
#[macro_export]
macro_rules! indicator_values {
    ($buffer:expr) => {
        &$buffer.items
    };
}
