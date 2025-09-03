//! Block-utility helpers (reward totals), written to be production-safe.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reward {
    pub amount: u64,
}

impl Reward {
    #[inline]
    pub const fn new(amount: u64) -> Self {
        Self { amount }
    }
}

pub mod rewards {
    use super::Reward;

    /// Total rewards as an integer (no precision loss).
    #[inline]
    pub fn total_u64(rewards: &[Reward]) -> u64 {
        rewards.iter().map(|r| r.amount).sum::<u64>()
    }

    /// Total rewards as a floating value (use only if you must emit f64).
    #[inline]
    pub fn total_f64(rewards: &[Reward]) -> f64 {
        rewards.iter().map(|r| r.amount as f64).sum::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::rewards::*;
    use super::*;

    #[test]
    fn sums_rewards_correctly() {
        let rs = [Reward::new(10), Reward::new(20), Reward::new(30)];
        assert_eq!(total_u64(&rs), 60);
        assert!((total_f64(&rs) - 60.0).abs() < f64::EPSILON);
    }
}
