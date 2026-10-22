/// Evaluates whether a chat's token count has crossed its compaction threshold (default 0.75).
pub fn should_compact(total_tokens: i64, context_window: i64, threshold: f64) -> bool {
    if context_window <= 0 {
        return false;
    }
    let ratio = (total_tokens as f64) / (context_window as f64);
    ratio >= threshold
}
