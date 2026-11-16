//! Login rate limiting.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use tokio::time::Instant;

use crate::errors::LoginError;

/// Failed logins per peer before the peer is locked out.
const MAX_PEER_FAILURES: u32 = 5;

/// Failed logins across all peers before every login is rejected.
const MAX_GLOBAL_FAILURES: u32 = 50;

/// Sliding failure window.
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Upper bound on tracked peer addresses.
const MAX_TRACKED_PEERS: usize = 4096;

#[derive(Default)]
pub(super) struct Limiter {
    peers: HashMap<IpAddr, Bucket>,
    unknown: Bucket,
    global: Bucket,
}

impl Limiter {
    pub(super) fn check(&self, peer: Option<IpAddr>, now: Instant) -> Result<(), LoginError> {
        let peer_bucket = peer
            .and_then(|address| self.peers.get(&address))
            .unwrap_or(&self.unknown);

        let retry_after = match (
            excess(peer_bucket, MAX_PEER_FAILURES, now),
            excess(&self.global, MAX_GLOBAL_FAILURES, now),
        ) {
            (Some(peer), Some(global)) => Some(peer.max(global)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        };

        match retry_after {
            Some(retry_after) => Err(LoginError::TooManyAttempts { retry_after }),
            None => Ok(()),
        }
    }

    pub(super) fn record_failure(&mut self, peer: Option<IpAddr>, now: Instant) {
        self.global.record_failure(now);

        let Some(peer) = peer else {
            self.unknown.record_failure(now);
            return;
        };

        if self.peers.len() >= MAX_TRACKED_PEERS {
            self.peers
                .retain(|_, bucket| bucket.retry_after(now).is_some());
        }

        // The global bucket still throttles a flood of new peers when the
        // per-peer table is full.
        if self.peers.len() < MAX_TRACKED_PEERS {
            self.peers.entry(peer).or_default().record_failure(now);
        }
    }

    pub(super) fn reset(&mut self, peer: Option<IpAddr>) {
        match peer {
            Some(peer) => {
                self.peers.remove(&peer);
            }
            None => self.unknown.reset(),
        }
    }
}

fn excess(bucket: &Bucket, limit: u32, now: Instant) -> Option<Duration> {
    (bucket.failures >= limit)
        .then(|| bucket.retry_after(now))
        .flatten()
}

#[derive(Default)]
struct Bucket {
    failures: u32,
    window_start: Option<Instant>,
}

impl Bucket {
    fn retry_after(&self, now: Instant) -> Option<Duration> {
        let start = self.window_start?;
        let elapsed = now.saturating_duration_since(start);
        (elapsed < FAILURE_WINDOW).then(|| FAILURE_WINDOW - elapsed)
    }

    fn record_failure(&mut self, now: Instant) {
        match self.window_start {
            Some(start) if now.saturating_duration_since(start) < FAILURE_WINDOW => {
                self.failures = self.failures.saturating_add(1);
            }
            _ => {
                self.window_start = Some(now);
                self.failures = 1;
            }
        }
    }

    fn reset(&mut self) {
        self.failures = 0;
        self.window_start = None;
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::{Limiter, MAX_TRACKED_PEERS};
    use tokio::time::Instant;

    #[test]
    fn tracked_peers_stay_capped_under_a_flood() {
        let now = Instant::now();
        let mut limiter = Limiter::default();

        for index in 0..=MAX_TRACKED_PEERS {
            let index = u32::try_from(index).expect("peer index fits u32");
            let peer = IpAddr::from(index.to_be_bytes());
            limiter.record_failure(Some(peer), now);
        }

        assert!(limiter.peers.len() <= MAX_TRACKED_PEERS);
    }
}
