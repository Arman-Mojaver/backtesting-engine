use crate::domain::signal_group::SignalGroup;

pub fn signal_groups_key(instrument: &str) -> String {
    format!("signal_groups:{}", instrument)
}

/// Lists all signal group cache keys currently stored in Redis.
///
/// Uses `KEYS signal_groups:*` — acceptable for a dev CLI tool with a small key space.
pub fn list_signal_group_keys(conn: &mut redis::Connection) -> anyhow::Result<Vec<String>> {
    let keys: Vec<String> = redis::cmd("KEYS").arg("signal_groups:*").query(conn)?;
    Ok(keys)
}

/// Reads signal groups from Redis. Returns `Ok(None)` if the key does not exist.
pub fn read_signal_groups(
    conn: &mut redis::Connection,
    key: &str,
) -> anyhow::Result<Option<Vec<SignalGroup>>> {
    let value: Option<String> = redis::cmd("GET").arg(key).query(conn)?;

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
    key: &str,
    groups: &Vec<SignalGroup>,
) -> anyhow::Result<()> {
    let json_str = serde_json::to_string(groups)
        .map_err(|e| anyhow::anyhow!("Failed to serialize signal groups: {}", e))?;

    redis::cmd("SET").arg(key).arg(json_str).query::<()>(conn)?;

    Ok(())
}

/// Deletes signal groups from Redis.
pub fn delete_signal_groups(conn: &mut redis::Connection, key: &str) -> anyhow::Result<()> {
    redis::cmd("DEL").arg(key).query::<()>(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_groups_key() {
        assert_eq!(signal_groups_key("EURUSD"), "signal_groups:EURUSD");
        assert_eq!(signal_groups_key("USDJPY"), "signal_groups:USDJPY");
    }

    #[test]
    fn test_signal_group_serde_roundtrip() {
        let groups: Vec<SignalGroup> = vec![
            SignalGroup {
                indicator_id: 1,
                long_signals: vec![100, 200, 300],
                short_signals: vec![150, 250],
                operation_count: 5,
                instrument: "EURUSD".to_string(),
            },
            SignalGroup {
                indicator_id: 2,
                long_signals: vec![],
                short_signals: vec![400],
                operation_count: 1,
                instrument: "EURUSD".to_string(),
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
            operation_count: 0,
            instrument: "EURUSD".to_string(),
        }];

        let json = serde_json::to_string(&groups).expect("serialize");
        let deserialized: Vec<SignalGroup> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized[0].indicator_id, 42);
        assert!(deserialized[0].long_signals.is_empty());
        assert!(deserialized[0].short_signals.is_empty());
    }
}
