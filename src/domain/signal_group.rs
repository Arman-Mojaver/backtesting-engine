use serde::{Deserialize, Serialize};

/// A group of long and short signal timestamps for a single indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalGroup {
    pub indicator_id: i32,
    pub long_signals: Vec<i64>,
    pub short_signals: Vec<i64>,
}
