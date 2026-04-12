use std::time::Duration;

/// Exponential backoff with jitter for sync error handling.
pub struct SyncBackoff {
    consecutive_failures: u32,
    base_delay: Duration,
    max_delay: Duration,
    circuit_break_threshold: u32,
}

impl SyncBackoff {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            base_delay: Duration::from_secs(15),
            max_delay: Duration::from_secs(300), // 5 minutes cap
            circuit_break_threshold: 5,
        }
    }

    /// Record a failure. Returns the delay to wait before retrying.
    pub fn record_failure(&mut self) -> Duration {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.current_delay()
    }

    /// Record a success. Resets the failure counter.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Whether the circuit breaker is open (too many consecutive failures).
    pub fn is_circuit_open(&self) -> bool {
        self.consecutive_failures >= self.circuit_break_threshold
    }

    /// Number of consecutive failures.
    pub fn failure_count(&self) -> u32 {
        self.consecutive_failures
    }

    /// Calculate the current backoff delay with jitter.
    pub fn current_delay(&self) -> Duration {
        if self.consecutive_failures == 0 {
            return Duration::ZERO;
        }
        let exp = (self.consecutive_failures - 1).min(10);
        let delay_secs = self.base_delay.as_secs_f64() * 2.0_f64.powi(exp as i32);
        let capped = delay_secs.min(self.max_delay.as_secs_f64());
        // Random jitter ±25%
        use rand::Rng;
        let jitter_factor = rand::thread_rng().gen_range(0.75..=1.25);
        Duration::from_secs_f64(capped * jitter_factor)
    }
}

impl Default for SyncBackoff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let b = SyncBackoff::new();
        assert_eq!(b.failure_count(), 0);
        assert!(!b.is_circuit_open());
    }

    #[test]
    fn delay_increases_on_failure() {
        let mut b = SyncBackoff::new();
        let d1 = b.record_failure();
        let d2 = b.record_failure();
        let d3 = b.record_failure();
        assert!(d2 > d1, "d2={d2:?} should be > d1={d1:?}");
        assert!(d3 > d2, "d3={d3:?} should be > d2={d2:?}");
    }

    #[test]
    fn delay_capped_at_max() {
        let mut b = SyncBackoff::new();
        for _ in 0..20 {
            b.record_failure();
        }
        let delay = b.current_delay();
        assert!(delay.as_secs() <= 375, "delay {delay:?} should be <= 375s");
    }

    #[test]
    fn success_resets() {
        let mut b = SyncBackoff::new();
        b.record_failure();
        b.record_failure();
        b.record_success();
        assert_eq!(b.failure_count(), 0);
        assert!(!b.is_circuit_open());
    }

    #[test]
    fn circuit_opens_at_threshold() {
        let mut b = SyncBackoff::new();
        for _ in 0..4 {
            b.record_failure();
            assert!(!b.is_circuit_open());
        }
        b.record_failure(); // 5th
        assert!(b.is_circuit_open());
    }
}
