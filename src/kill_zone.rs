// ─────────────────────────────────────────────────────────────────────────────
// kill_zone.rs  –  Kill-zone (session) gating
// ─────────────────────────────────────────────────────────────────────────────

use crate::config::KILL_ZONES;
use chrono::{DateTime, Timelike, Utc};

/// Returns `true` when the given UTC timestamp falls inside any configured
/// kill-zone window.
///
/// Kill-zones are defined in `config::KILL_ZONES` as `(start_hour, end_hour)`
/// pairs in UTC, where the end is **exclusive**.
pub fn is_kill_zone(ts_ms: i64) -> bool {
    // Convert milliseconds → UTC DateTime
    let secs  = ts_ms / 1_000;
    let nanos = ((ts_ms % 1_000) * 1_000_000) as u32;
    let dt: DateTime<Utc> = DateTime::from_timestamp(secs, nanos)
        .unwrap_or_else(Utc::now);

    let utc_hour = dt.hour();
    KILL_ZONES
        .iter()
        .any(|&(start, end)| utc_hour >= start && utc_hour < end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ny_session_is_active() {
        // 2024-01-15 13:00:00 UTC  →  08:00 EST  (inside NY 12–15 UTC window)
        let ts_ms = 1_705_320_000_000_i64;
        assert!(is_kill_zone(ts_ms));
    }

    #[test]
    fn dead_of_night_is_inactive() {
        // 2024-01-15 02:00:00 UTC  →  outside both windows
        let ts_ms = 1_705_284_000_000_i64;
        assert!(!is_kill_zone(ts_ms));
    }
}
