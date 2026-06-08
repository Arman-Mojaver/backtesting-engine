use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A group of long and short signal timestamps for a single indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalGroup {
    pub long_signals: Vec<i64>,
    pub short_signals: Vec<i64>,
}

/// Maps indicator IDs to their signal groups.
pub type SignalGroups = HashMap<i32, SignalGroup>;

/// The Redis key for the signal groups cache.
pub const SIGNAL_GROUPS_KEY: &str = "signal_groups";

/// Reads signal groups from Redis. Returns `Ok(None)` if the key does not exist.
pub fn read_signal_groups(conn: &mut redis::Connection) -> anyhow::Result<Option<SignalGroups>> {
    let value: Option<String> = redis::cmd("GET").arg(SIGNAL_GROUPS_KEY).query(conn)?;

    match value {
        Some(json_str) => {
            let groups: SignalGroups = serde_json::from_str(&json_str)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize signal groups: {}", e))?;
            Ok(Some(groups))
        }
        None => Ok(None),
    }
}

/// Writes signal groups to Redis, overwriting any existing value.
pub fn write_signal_groups(
    conn: &mut redis::Connection,
    groups: &SignalGroups,
) -> anyhow::Result<()> {
    let json_str = serde_json::to_string(groups)
        .map_err(|e| anyhow::anyhow!("Failed to serialize signal groups: {}", e))?;

    redis::cmd("SET")
        .arg(SIGNAL_GROUPS_KEY)
        .arg(json_str)
        .query::<()>(conn)?;

    Ok(())
}

/// Deletes signal groups from Redis.
pub fn delete_signal_groups(conn: &mut redis::Connection) -> anyhow::Result<()> {
    redis::cmd("DEL").arg(SIGNAL_GROUPS_KEY).query::<()>(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_group_serde_roundtrip() {
        let groups: SignalGroups = HashMap::from([
            (
                1,
                SignalGroup {
                    long_signals: vec![100, 200, 300],
                    short_signals: vec![150, 250],
                },
            ),
            (
                2,
                SignalGroup {
                    long_signals: vec![],
                    short_signals: vec![400],
                },
            ),
        ]);

        let json = serde_json::to_string(&groups).expect("serialize");
        let deserialized: SignalGroups = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(
            deserialized.get(&1).unwrap().long_signals,
            vec![100, 200, 300]
        );
        assert_eq!(deserialized.get(&1).unwrap().short_signals, vec![150, 250]);
        assert!(deserialized.get(&2).unwrap().long_signals.is_empty());
        assert_eq!(deserialized.get(&2).unwrap().short_signals, vec![400]);
    }

    #[test]
    fn test_signal_group_empty_vectors() {
        let groups: SignalGroups = HashMap::from([(
            42,
            SignalGroup {
                long_signals: vec![],
                short_signals: vec![],
            },
        )]);

        let json = serde_json::to_string(&groups).expect("serialize");
        let deserialized: SignalGroups = serde_json::from_str(&json).expect("deserialize");

        assert!(deserialized.get(&42).unwrap().long_signals.is_empty());
        assert!(deserialized.get(&42).unwrap().short_signals.is_empty());
    }

    #[test]
    fn test_signal_groups_key_constant() {
        assert_eq!(SIGNAL_GROUPS_KEY, "signal_groups");
    }
}
