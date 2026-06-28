use crate::domain::signal_group::SignalGroup;

/// The Redis key for the signal groups cache.
pub const SIGNAL_GROUPS_KEY: &str = "signal_groups";

/// Reads signal groups from Redis. Returns `Ok(None)` if the key does not exist.
pub fn read_signal_groups(
    conn: &mut redis::Connection,
) -> anyhow::Result<Option<Vec<SignalGroup>>> {
    let value: Option<String> = redis::cmd("GET").arg(SIGNAL_GROUPS_KEY).query(conn)?;

    match value {
        Some(json_str) => {
            let groups: Vec<SignalGroup> = serde_json::from_str(&json_str)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize signal groups: {}", e))?;
            Ok(Some(groups))
        }
        None => Ok(None),
    }
}

/// Writes signal groups to Redis, overwriting any existing value.
pub fn write_signal_groups(
    conn: &mut redis::Connection,
    groups: &Vec<SignalGroup>,
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
        let groups: Vec<SignalGroup> = vec![
            SignalGroup {
                indicator_id: 1,
                long_signals: vec![100, 200, 300],
                short_signals: vec![150, 250],
            },
            SignalGroup {
                indicator_id: 2,
                long_signals: vec![],
                short_signals: vec![400],
            },
        ];

        let json = serde_json::to_string(&groups).expect("serialize");
        let deserialized: Vec<SignalGroup> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].indicator_id, 1);
        assert_eq!(deserialized[0].long_signals, vec![100, 200, 300]);
        assert_eq!(deserialized[0].short_signals, vec![150, 250]);
        assert_eq!(deserialized[1].indicator_id, 2);
        assert!(deserialized[1].long_signals.is_empty());
        assert_eq!(deserialized[1].short_signals, vec![400]);
    }

    #[test]
    fn test_signal_group_empty_vectors() {
        let groups: Vec<SignalGroup> = vec![SignalGroup {
            indicator_id: 42,
            long_signals: vec![],
            short_signals: vec![],
        }];

        let json = serde_json::to_string(&groups).expect("serialize");
        let deserialized: Vec<SignalGroup> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized[0].indicator_id, 42);
        assert!(deserialized[0].long_signals.is_empty());
        assert!(deserialized[0].short_signals.is_empty());
    }

    #[test]
    fn test_signal_groups_key_constant() {
        assert_eq!(SIGNAL_GROUPS_KEY, "signal_groups");
    }
}
