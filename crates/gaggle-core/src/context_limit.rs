//! Context limit helpers (Phase 3).
//!
//! Small, pure helpers for combining ratio-based and absolute token caps when
//! deciding whether context compaction should run.

/// Returns true when `total_tokens` exceeds either the configured ratio of
/// `context_window` or the absolute cap (default 225_000 tokens).
pub fn exceeds_context_limit(total_tokens: i64, context_window: i64, threshold: f64) -> bool {
    if context_window <= 0 {
        return total_tokens >= 225_000;
    }
    let ratio_limit = (context_window as f64 * threshold) as i64;
    total_tokens >= ratio_limit.max(1) || total_tokens >= 225_000
}

/// Clamp a requested context window to a sane minimum/maximum range.
pub fn clamp_context_window(requested: i64) -> i64 {
    requested.clamp(1_024, 2_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_or_absolute() {
        assert!(exceeds_context_limit(200_000, 100_000, 0.75));
        assert!(exceeds_context_limit(230_000, 1_000_000, 0.75));
        assert!(!exceeds_context_limit(10_000, 100_000, 0.75));
    }

    #[test]
    fn clamp_range() {
        assert_eq!(clamp_context_window(0), 1_024);
        assert_eq!(clamp_context_window(100), 1_024);
        assert_eq!(clamp_context_window(50_000), 50_000);
        assert_eq!(clamp_context_window(10_000_000), 2_000_000);
    }
}
