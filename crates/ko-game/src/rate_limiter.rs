//! Rate limiter for flood protection — throttles packets per session/IP.
//!
//! Uses a fixed window counter algorithm to enforce:
//! - Per-IP connection limit (max 5 concurrent connections)
//! - Per-session packet rate: soft limit (100 pps, drop packets), critical limit (300 pps, disconnect)
//! - Per-opcode throttle (chat=5/s, move=20/s, attack=10/s, magic=40/s, other=30/s)
//! - Chat group limiter: WizChat, WizNationChat, WizChatTarget share a single counter
//! - GM bypass: sessions flagged as GM skip all rate limits
//! - Temporary ban after 3 violations (5 minutes)
//!
//! C++ has no equivalent — this is a server-hardening addition.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use ko_protocol::Opcode;
use thiserror::Error;
use tracing::warn;

use crate::zone::SessionId;

/// Rate limiting error types.
#[derive(Debug, Error)]
pub enum RateLimitError {
    /// Session exceeded the soft per-second packet limit (packets should be dropped).
    #[error("session {0} exceeded global packet rate ({1} packets/sec)")]
    GlobalRateExceeded(SessionId, u32),

    /// Session exceeded the critical per-second packet limit (session should be disconnected).
    #[error("session {0} exceeded CRITICAL packet rate ({1} packets/sec) — disconnect")]
    CriticalRateExceeded(SessionId, u32),

    /// Session exceeded per-opcode rate limit.
    #[error("session {0} exceeded opcode rate for {1:?} ({2} per sec)")]
    OpcodeRateExceeded(SessionId, Option<Opcode>, u32),

    /// IP has too many concurrent connections.
    #[error("IP {0} exceeded max concurrent connections ({1})")]
    TooManyConnections(IpAddr, u32),

    /// Session is temporarily banned after repeated violations.
    #[error("session {0} is temporarily banned until violation count resets")]
    TemporaryBan(SessionId),
}

impl RateLimitError {
    /// Returns `true` if this error requires the session to be disconnected
    /// rather than just dropping the current packet.
    pub fn should_disconnect(&self) -> bool {
        matches!(
            self,
            RateLimitError::CriticalRateExceeded(..) | RateLimitError::TemporaryBan(..)
        )
    }
}

/// Maximum concurrent connections per IP.
const MAX_CONNECTIONS_PER_IP: u32 = 5;

/// Soft limit: packets per second before we start dropping (generous for normal play).
///
/// Normal gameplay produces roughly 20-50 pps (movement + rotation + attacks + UI).
/// 100 pps gives ample headroom while catching obvious floods.
const SOFT_PACKETS_PER_SECOND: u32 = 100;

/// Critical limit: packets per second before we disconnect the session.
///
/// 300 pps is well beyond anything a legitimate client can produce and indicates
/// either a bot or a flood attack.
const CRITICAL_PACKETS_PER_SECOND: u32 = 300;

/// Maximum violations before temporary ban.
const MAX_VIOLATIONS: u32 = 3;

/// Duration of temporary ban after exceeding violation threshold.
const BAN_DURATION: Duration = Duration::from_secs(300); // 5 minutes

/// Sliding window duration for rate counting.
const WINDOW_DURATION: Duration = Duration::from_secs(1);

/// Synthetic group key for chat opcodes (must not collide with real opcode bytes).
///
/// WizChat, WizNationChat, and WizChatTarget share this single counter so a
/// player cannot exceed the chat rate by spreading messages across channels.
const CHAT_GROUP_KEY: u8 = 0xFF;

/// Get the per-opcode rate limit for a given opcode.
///
/// Returns the maximum allowed packets per second for the opcode.
/// Thresholds are generous enough for normal gameplay while catching abuse.
fn opcode_rate_limit(opcode: Option<Opcode>) -> u32 {
    match opcode {
        Some(Opcode::WizChat) | Some(Opcode::WizNationChat) | Some(Opcode::WizChatTarget) => 5,
        Some(Opcode::WizMove) => 20,
        Some(Opcode::WizAttack) => 10,
        // The v2615 client sends a burst of WIZ_MAGIC_PROCESS packets for one
        // legitimate archer cast: Arrow Shower / Multiple Shot use a cast
        // packet plus one pair of effect packets per projectile.  A single
        // five-arrow use produced 13 packets in the captured client log, so
        // the old 10/s limit both dropped damage packets and accumulated all
        // three ban strikes during that one cast.  Forty accepts two complete
        // multi-shot bursts across a one-second window boundary while the
        // existing 100 pps global and 300 pps critical guards still protect
        // the session from packet floods.
        Some(Opcode::WizMagicProcess) => 40,
        _ => 30,
    }
}

/// Map an opcode byte to a rate-limit counter key.
///
/// Chat opcodes (`WizChat`, `WizNationChat`, `WizChatTarget`) are mapped to a
/// shared [`CHAT_GROUP_KEY`] so their counters are combined. All other opcodes
/// use their raw byte value as the key.
fn opcode_counter_key(opcode_byte: u8) -> u8 {
    match Opcode::from_byte(opcode_byte) {
        Some(Opcode::WizChat) | Some(Opcode::WizNationChat) | Some(Opcode::WizChatTarget) => {
            CHAT_GROUP_KEY
        }
        _ => opcode_byte,
    }
}

/// Per-session rate tracking state.
struct SessionRateState {
    /// Timestamp of the current window start.
    window_start: Instant,
    /// Total packet count in the current window.
    global_count: u32,
    /// Per-opcode packet counts in the current window.
    opcode_counts: HashMap<u8, u32>,
    /// Number of consecutive violations.
    violation_count: u32,
    /// If banned, the instant when the ban expires.
    ban_until: Option<Instant>,
}

impl SessionRateState {
    /// Create a new session rate state.
    fn new() -> Self {
        Self {
            window_start: Instant::now(),
            global_count: 0,
            opcode_counts: HashMap::new(),
            violation_count: 0,
            ban_until: None,
        }
    }

    /// Reset the sliding window if it has elapsed.
    fn maybe_reset_window(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= WINDOW_DURATION {
            self.window_start = now;
            self.global_count = 0;
            self.opcode_counts.clear();
        }
    }

    /// Record a violation. Returns true if the session should be banned.
    fn record_violation(&mut self) -> bool {
        self.violation_count += 1;
        if self.violation_count >= MAX_VIOLATIONS {
            self.ban_until = Some(Instant::now() + BAN_DURATION);
            true
        } else {
            false
        }
    }

    /// Check if the session is currently banned.
    fn is_banned(&mut self) -> bool {
        if let Some(until) = self.ban_until {
            if Instant::now() >= until {
                // Ban expired — reset
                self.ban_until = None;
                self.violation_count = 0;
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}

/// Packet rate limiter for flood protection.
///
/// Thread-safe: uses `DashMap` for concurrent access from multiple session tasks.
///
/// # Rate limits
/// - **IP connections**: max 5 concurrent per IP
/// - **Session global (soft)**: 100 packets/second — excess packets are silently dropped
/// - **Session global (critical)**: 300 packets/second — session is disconnected
/// - **Per-opcode**: WIZ_CHAT=5/s, WIZ_MOVE=20/s, WIZ_ATTACK=10/s,
///   WIZ_MAGIC_PROCESS=40/s, other=30/s
/// - **Violations**: 3 strikes = 5 minute temporary ban
pub struct RateLimiter {
    /// Per-session rate state.
    sessions: DashMap<SessionId, SessionRateState>,
    /// Per-IP connection count.
    ip_connections: DashMap<IpAddr, AtomicU64>,
}

impl RateLimiter {
    /// Create a new rate limiter with empty state.
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            ip_connections: DashMap::new(),
        }
    }

    /// Register a new connection from an IP address.
    ///
    /// Returns `Err(RateLimitError::TooManyConnections)` if the IP already
    /// has `MAX_CONNECTIONS_PER_IP` active connections.
    pub fn register_connection(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        let entry = self
            .ip_connections
            .entry(ip)
            .or_insert_with(|| AtomicU64::new(0));
        let current = entry.value().fetch_add(1, Ordering::Relaxed) + 1;
        if current > MAX_CONNECTIONS_PER_IP as u64 {
            // Undo the increment
            entry.value().fetch_sub(1, Ordering::Relaxed);
            warn!(
                "Rate limit: IP {} exceeded max connections ({})",
                ip, MAX_CONNECTIONS_PER_IP
            );
            return Err(RateLimitError::TooManyConnections(
                ip,
                MAX_CONNECTIONS_PER_IP,
            ));
        }
        Ok(())
    }

    /// Unregister a connection from an IP address (on disconnect).
    ///
    /// Uses `DashMap::remove_if` to atomically decrement the counter and
    /// remove the entry when it reaches zero, avoiding a TOCTOU race between
    /// the decrement and the removal.
    pub fn unregister_connection(&self, ip: IpAddr) {
        self.ip_connections
            .remove_if(&ip, |_, count| count.fetch_sub(1, Ordering::Relaxed) <= 1);
    }

    /// Register a new session for rate tracking.
    pub fn register_session(&self, session_id: SessionId) {
        self.sessions.insert(session_id, SessionRateState::new());
    }

    /// Unregister a session on disconnect (frees memory).
    pub fn unregister_session(&self, session_id: SessionId) {
        self.sessions.remove(&session_id);
    }

    /// Check if a packet from a session should be allowed through.
    ///
    /// Returns `Ok(())` if the packet is allowed, or a `RateLimitError` if
    /// the rate limit has been exceeded. Callers should:
    /// - Drop the packet on soft errors (`GlobalRateExceeded`, `OpcodeRateExceeded`)
    /// - Disconnect the session on critical errors (`CriticalRateExceeded`, `TemporaryBan`)
    ///
    /// Use [`RateLimitError::should_disconnect`] to distinguish the two cases.
    ///
    /// When `is_gm` is `true` (authority == 0), all rate limits are bypassed
    /// so that GM commands are never throttled.
    pub fn check_rate_limit(
        &self,
        session_id: SessionId,
        opcode_byte: u8,
        is_gm: bool,
    ) -> Result<(), RateLimitError> {
        // GMs bypass all rate limits.
        if is_gm {
            return Ok(());
        }

        let opcode = Opcode::from_byte(opcode_byte);

        let mut entry = match self.sessions.get_mut(&session_id) {
            Some(e) => e,
            None => {
                // Session not registered — allow (shouldn't happen in practice)
                return Ok(());
            }
        };

        let state = entry.value_mut();

        // Check temporary ban first
        if state.is_banned() {
            return Err(RateLimitError::TemporaryBan(session_id));
        }

        // Reset window if elapsed
        state.maybe_reset_window();

        // Increment global count
        state.global_count += 1;

        // Critical threshold — session should be disconnected immediately
        if state.global_count > CRITICAL_PACKETS_PER_SECOND {
            let banned = state.record_violation();
            if banned {
                warn!(
                    "Rate limit: session {} BANNED for {} seconds (exceeded CRITICAL rate {} times)",
                    session_id,
                    BAN_DURATION.as_secs(),
                    MAX_VIOLATIONS
                );
            }
            return Err(RateLimitError::CriticalRateExceeded(
                session_id,
                CRITICAL_PACKETS_PER_SECOND,
            ));
        }

        // Soft threshold — packet is silently dropped, but session stays connected.
        // Does NOT record a violation (could be a brief burst from legitimate play).
        // Violations are only recorded for per-opcode abuse and critical floods.
        if state.global_count > SOFT_PACKETS_PER_SECOND {
            return Err(RateLimitError::GlobalRateExceeded(
                session_id,
                SOFT_PACKETS_PER_SECOND,
            ));
        }

        // Increment per-opcode count (using group key for chat opcodes)
        let counter_key = opcode_counter_key(opcode_byte);
        let opcode_count = state.opcode_counts.entry(counter_key).or_insert(0);
        *opcode_count += 1;
        let limit = opcode_rate_limit(opcode);
        if *opcode_count > limit {
            let banned = state.record_violation();
            if banned {
                warn!(
                    "Rate limit: session {} BANNED for {} seconds (exceeded {:?} rate {} times)",
                    session_id,
                    BAN_DURATION.as_secs(),
                    opcode,
                    MAX_VIOLATIONS
                );
            }
            return Err(RateLimitError::OpcodeRateExceeded(
                session_id, opcode, limit,
            ));
        }

        Ok(())
    }

    /// Get the current violation count for a session (for testing/monitoring).
    pub fn violation_count(&self, session_id: SessionId) -> u32 {
        self.sessions
            .get(&session_id)
            .map(|e| e.violation_count)
            .unwrap_or(0)
    }

    /// Check if a session is currently banned (for testing/monitoring).
    pub fn is_banned(&self, session_id: SessionId) -> bool {
        self.sessions
            .get_mut(&session_id)
            .is_some_and(|mut e| e.is_banned())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    const TEST_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    #[test]
    fn test_normal_traffic_passes() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Send a mix of packets within all limits: 20 move + 10 attack + 30 rotate = 60 total
        // All within per-opcode limits and well under the 100 pps soft limit.
        for _ in 0..20 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizMove as u8, false)
                .is_ok());
        }
        for _ in 0..10 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizAttack as u8, false)
                .is_ok());
        }
        for _ in 0..30 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizRotate as u8, false)
                .is_ok());
        }
    }

    #[test]
    fn test_soft_global_rate_exceeded() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Send 100 packets (soft limit) with diverse opcodes to avoid opcode limit.
        // WizMove limit = 20, WizAttack = 10, WizRotate(other) = 30,
        // WizStateChange(other) = 30 → 20+10+30+30 = 90 within opcode limits.
        // Add 10 more using WizTargetHp(other) to reach 100.
        for _ in 0..20 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizMove as u8, false)
                .is_ok());
        }
        for _ in 0..10 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizAttack as u8, false)
                .is_ok());
        }
        for _ in 0..30 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizRotate as u8, false)
                .is_ok());
        }
        for _ in 0..30 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizStateChange as u8, false)
                .is_ok());
        }
        for _ in 0..10 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizTargetHp as u8, false)
                .is_ok());
        }

        // 101st packet should fail (soft global rate exceeded)
        let result = limiter.check_rate_limit(1, Opcode::WizReqUserIn as u8, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RateLimitError::GlobalRateExceeded(1, _)));
        assert!(!err.should_disconnect()); // soft limit — do NOT disconnect
    }

    #[test]
    fn test_critical_global_rate_exceeded() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Pump 300 packets to reach the critical threshold.
        // Use many different opcodes to stay under per-opcode limits.
        // We'll use 10 distinct "other" opcodes at 30 each = 300.
        let other_opcodes = [
            Opcode::WizRotate as u8,
            Opcode::WizStateChange as u8,
            Opcode::WizTargetHp as u8,
            Opcode::WizReqUserIn as u8,
            Opcode::WizReqNpcIn as u8,
            Opcode::WizZoneChange as u8,
            Opcode::WizItemMove as u8,
            Opcode::WizSkillData as u8,
            Opcode::WizHelmet as u8,
            Opcode::WizGenie as u8,
        ];
        for &op in &other_opcodes {
            for _ in 0..30 {
                let _ = limiter.check_rate_limit(1, op, false);
            }
        }

        // Packets beyond 100 were dropped (soft limit), but session stays.
        // Now send packet 301 — should be CriticalRateExceeded.
        let result = limiter.check_rate_limit(1, Opcode::WizDead as u8, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RateLimitError::CriticalRateExceeded(1, _)));
        assert!(err.should_disconnect()); // critical limit — MUST disconnect
    }

    #[test]
    fn test_chat_opcode_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // WIZ_CHAT limit = 5/s
        for _ in 0..5 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizChat as u8, false)
                .is_ok());
        }

        // 6th chat in same window should fail
        let result = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizChat), 5)
        ));
    }

    #[test]
    fn test_attack_opcode_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // WIZ_ATTACK limit = 10/s
        for _ in 0..10 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizAttack as u8, false)
                .is_ok());
        }

        // 11th should fail
        let result = limiter.check_rate_limit(1, Opcode::WizAttack as u8, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizAttack), 10)
        ));
    }

    #[test]
    fn test_magic_opcode_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // v2615 multi-arrow skills legitimately burst 13 packets per cast.
        // Keep enough room for two casts around the fixed-window boundary.
        for _ in 0..40 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizMagicProcess as u8, false)
                .is_ok());
        }

        // The 41st packet in the same one-second window is still rejected.
        let result = limiter.check_rate_limit(1, Opcode::WizMagicProcess as u8, false);
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizMagicProcess), 40)
        ));
    }

    #[test]
    fn test_move_opcode_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // WIZ_MOVE limit = 20/s
        for _ in 0..20 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizMove as u8, false)
                .is_ok());
        }

        // 21st should fail
        let result = limiter.check_rate_limit(1, Opcode::WizMove as u8, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizMove), 20)
        ));
    }

    #[test]
    fn test_ip_connection_limit() {
        let limiter = RateLimiter::new();

        // Register 5 connections from same IP — should pass
        for _ in 0..5 {
            assert!(limiter.register_connection(TEST_IP).is_ok());
        }

        // 6th should fail
        let result = limiter.register_connection(TEST_IP);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::TooManyConnections(_, 5)
        ));
    }

    #[test]
    fn test_ip_connection_unregister_allows_new() {
        let limiter = RateLimiter::new();

        // Fill up to limit
        for _ in 0..5 {
            assert!(limiter.register_connection(TEST_IP).is_ok());
        }

        // Disconnect one
        limiter.unregister_connection(TEST_IP);

        // Now one more should work
        assert!(limiter.register_connection(TEST_IP).is_ok());
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new();
        let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        // 5 connections from ip1
        for _ in 0..5 {
            assert!(limiter.register_connection(ip1).is_ok());
        }

        // ip2 should still be able to connect
        assert!(limiter.register_connection(ip2).is_ok());
    }

    #[test]
    fn test_temporary_ban_after_violations() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Trigger 3 violations by exceeding chat limit (5/s) 3 times
        // Each window: send 5 OK + 1 violation, then force window reset
        for _ in 0..3 {
            // Reset window
            limiter.sessions.get_mut(&1).unwrap().window_start = Instant::now() - WINDOW_DURATION;
            // Send 5 OK + 1 violation
            for _ in 0..5 {
                let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
            }
            let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false); // violation
        }

        // Should now be banned
        assert!(limiter.is_banned(1));

        // Any packet should return TemporaryBan
        let result = limiter.check_rate_limit(1, Opcode::WizMove as u8, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::TemporaryBan(1)
        ));
    }

    #[test]
    fn test_violation_count_tracking() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // No violations initially
        assert_eq!(limiter.violation_count(1), 0);

        // Trigger one violation: send 5 OK + 1 over-limit (chat limit = 5/s)
        for _ in 0..5 {
            let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
        }
        let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false); // violation

        assert_eq!(limiter.violation_count(1), 1);
    }

    #[test]
    fn test_window_reset_allows_new_packets() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Fill up chat limit (5/s)
        for _ in 0..5 {
            let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
        }
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChat as u8, false)
            .is_err());

        // Manually expire the window
        limiter.sessions.get_mut(&1).unwrap().window_start = Instant::now() - WINDOW_DURATION;

        // Should work again
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChat as u8, false)
            .is_ok());
    }

    #[test]
    fn test_unregistered_session_passes() {
        let limiter = RateLimiter::new();
        // Session 999 never registered — should pass through
        assert!(limiter
            .check_rate_limit(999, Opcode::WizMove as u8, false)
            .is_ok());
    }

    #[test]
    fn test_session_cleanup_frees_memory() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);
        assert!(limiter.sessions.contains_key(&1));

        limiter.unregister_session(1);
        assert!(!limiter.sessions.contains_key(&1));
    }

    #[test]
    fn test_nation_chat_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // WIZ_NATION_CHAT shares the chat group counter (5/s total across all chat types)
        for _ in 0..5 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizNationChat as u8, false)
                .is_ok());
        }
        assert!(limiter
            .check_rate_limit(1, Opcode::WizNationChat as u8, false)
            .is_err());
    }

    #[test]
    fn test_chat_target_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // WIZ_CHAT_TARGET shares the chat group counter (5/s total across all chat types)
        for _ in 0..5 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizChatTarget as u8, false)
                .is_ok());
        }
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChatTarget as u8, false)
            .is_err());
    }

    /// Verify that chat opcodes share a single counter: 2 WizChat + 2 WizNationChat + 1 WizChatTarget = 5,
    /// so a 6th message via WizChatTarget is blocked.
    #[test]
    fn test_chat_group_shared_counter() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // 2 chats via WizChat
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChat as u8, false)
            .is_ok());
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChat as u8, false)
            .is_ok());
        // 2 chats via WizNationChat — shares the same group counter (total=4)
        assert!(limiter
            .check_rate_limit(1, Opcode::WizNationChat as u8, false)
            .is_ok());
        assert!(limiter
            .check_rate_limit(1, Opcode::WizNationChat as u8, false)
            .is_ok());
        // 5th via WizChatTarget — should pass (total=5)
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChatTarget as u8, false)
            .is_ok());
        // 6th message via WizChatTarget — should be blocked (group counter = 5/5)
        let result = limiter.check_rate_limit(1, Opcode::WizChatTarget as u8, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizChatTarget), 5)
        ));
    }

    /// Verify that `opcode_counter_key` maps all chat opcodes to the same key.
    #[test]
    fn test_opcode_counter_key_chat_group() {
        let chat_key = opcode_counter_key(Opcode::WizChat as u8);
        let nation_key = opcode_counter_key(Opcode::WizNationChat as u8);
        let target_key = opcode_counter_key(Opcode::WizChatTarget as u8);

        assert_eq!(chat_key, CHAT_GROUP_KEY);
        assert_eq!(nation_key, CHAT_GROUP_KEY);
        assert_eq!(target_key, CHAT_GROUP_KEY);

        // Non-chat opcodes use their raw byte
        assert_eq!(
            opcode_counter_key(Opcode::WizMove as u8),
            Opcode::WizMove as u8
        );
        assert_eq!(
            opcode_counter_key(Opcode::WizAttack as u8),
            Opcode::WizAttack as u8
        );
    }

    /// GM sessions (is_gm = true) bypass all rate limits.
    #[test]
    fn test_gm_bypass_rate_limit() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Send 150 chat packets as GM — all should pass (well beyond the 5/s limit)
        for _ in 0..150 {
            assert!(limiter
                .check_rate_limit(1, Opcode::WizChat as u8, true)
                .is_ok());
        }

        // Also exceed global limit (100/s) — still fine for GM
        // (150 already sent, no failure)
    }

    /// GM bypass works even when the session is banned.
    #[test]
    fn test_gm_bypass_even_when_banned() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);

        // Get the session banned via non-GM traffic (exceed chat 5/s limit 3 times)
        for _ in 0..3 {
            limiter.sessions.get_mut(&1).unwrap().window_start = Instant::now() - WINDOW_DURATION;
            for _ in 0..5 {
                let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
            }
            let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false); // violation
        }
        assert!(limiter.is_banned(1));

        // GM flag still bypasses the ban
        assert!(limiter
            .check_rate_limit(1, Opcode::WizMove as u8, true)
            .is_ok());
    }

    /// TOCTOU fix: `unregister_connection` atomically removes entry at zero.
    #[test]
    fn test_unregister_connection_removes_entry_at_zero() {
        let limiter = RateLimiter::new();
        assert!(limiter.register_connection(TEST_IP).is_ok());

        // Single connection → unregister should remove the entry entirely
        limiter.unregister_connection(TEST_IP);
        assert!(!limiter.ip_connections.contains_key(&TEST_IP));
    }

    /// Multiple unregisters decrement correctly without removing prematurely.
    #[test]
    fn test_unregister_connection_multiple_decrement() {
        let limiter = RateLimiter::new();
        // Register 3 connections
        for _ in 0..3 {
            assert!(limiter.register_connection(TEST_IP).is_ok());
        }

        // Unregister 1 — should still have entry (count = 2)
        limiter.unregister_connection(TEST_IP);
        assert!(limiter.ip_connections.contains_key(&TEST_IP));

        // Unregister 1 more — count = 1, entry should still exist
        limiter.unregister_connection(TEST_IP);
        assert!(limiter.ip_connections.contains_key(&TEST_IP));

        // Unregister last — entry should be removed
        limiter.unregister_connection(TEST_IP);
        assert!(!limiter.ip_connections.contains_key(&TEST_IP));
    }

    #[test]
    fn test_default_trait() {
        let limiter = RateLimiter::default();
        assert!(limiter.sessions.is_empty());
        assert!(limiter.ip_connections.is_empty());
    }

    #[test]
    fn test_opcode_rate_limit_values() {
        assert_eq!(opcode_rate_limit(Some(Opcode::WizChat)), 5);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizNationChat)), 5);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizChatTarget)), 5);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizMove)), 20);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizAttack)), 10);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizMagicProcess)), 40);
        assert_eq!(opcode_rate_limit(Some(Opcode::WizDead)), 30); // "other"
        assert_eq!(opcode_rate_limit(None), 30);
    }

    #[test]
    fn test_multiple_sessions_independent() {
        let limiter = RateLimiter::new();
        limiter.register_session(1);
        limiter.register_session(2);

        // Fill session 1's chat limit (5/s)
        for _ in 0..5 {
            let _ = limiter.check_rate_limit(1, Opcode::WizChat as u8, false);
        }
        assert!(limiter
            .check_rate_limit(1, Opcode::WizChat as u8, false)
            .is_err());

        // Session 2 should still be fine
        assert!(limiter
            .check_rate_limit(2, Opcode::WizChat as u8, false)
            .is_ok());
    }

    /// `should_disconnect` returns false for soft errors and true for critical errors.
    #[test]
    fn test_should_disconnect_classification() {
        // Soft errors — do NOT disconnect
        let soft_global = RateLimitError::GlobalRateExceeded(1, 100);
        assert!(!soft_global.should_disconnect());

        let opcode_err = RateLimitError::OpcodeRateExceeded(1, Some(Opcode::WizChat), 5);
        assert!(!opcode_err.should_disconnect());

        let ip_err = RateLimitError::TooManyConnections(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5);
        assert!(!ip_err.should_disconnect());

        // Critical errors — MUST disconnect
        let critical = RateLimitError::CriticalRateExceeded(1, 300);
        assert!(critical.should_disconnect());

        let banned = RateLimitError::TemporaryBan(1);
        assert!(banned.should_disconnect());
    }
}
