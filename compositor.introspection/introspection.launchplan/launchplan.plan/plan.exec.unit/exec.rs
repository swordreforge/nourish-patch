//! Unit-name helpers for systemd-managed launches.

/// Strip characters systemd disallows in unit names. Allowed:
/// `[A-Za-z0-9:-_.\]`. Everything else becomes `_`. Empty input
/// becomes `"app"` so we always have *something* before the random
/// suffix.
pub fn sanitise_unit_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "app".to_string()
    } else {
        cleaned
    }
}

/// Generate a short random hex suffix for unit names. Uses a
/// process-local counter mixed with subsecond nanos, so no extra
/// crate needed. Collisions would be cosmetic (systemd would reject
/// the duplicate unit, the launch would just fail and the user retries).
pub fn short_random() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    // Mix in nanos so two launches in different processes don't
    // collide (since the counter is per-process). Truncate to 6
    // hex chars — 24 bits of entropy is overkill for "won't collide
    // with the previous launch of the same app within seconds".
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    format!("{:06x}", (nanos ^ count) & 0xff_ffff)
}
