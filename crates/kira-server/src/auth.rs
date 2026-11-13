//! PIN authentication, signed session cookies, and login rate limiting.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use kira_config::{SameSite, ServerConfig};
use ring::digest::{SHA256, digest};
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use subtle::ConstantTimeEq;
use tokio::time::Instant;

use crate::ServerError;

/// Session cookie name.
pub const COOKIE_NAME: &str = "kira_session";

/// Session lifetime.
pub const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Failed logins per peer before the peer is locked out.
const MAX_PEER_FAILURES: u32 = 5;

/// Failed logins across all peers before every login is rejected.
const MAX_GLOBAL_FAILURES: u32 = 50;

/// Sliding failure window.
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Upper bound on tracked peer addresses.
const MAX_TRACKED_PEERS: usize = 4096;

const TOKEN_VERSION: &str = "v1";

/// Loopback dashboard origins allowed when none are configured.
const DEFAULT_ORIGINS: [&str; 4] = [
    "http://127.0.0.1:3000",
    "http://localhost:3000",
    "http://127.0.0.1:5173",
    "http://localhost:5173",
];

/// Authentication settings for the server.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Configured PIN or password. Generated when absent.
    pub password: Option<String>,
    /// Exact dashboard origins allowed to call the API.
    pub allowed_origins: Vec<String>,
    /// Set the `Secure` cookie attribute.
    pub cookie_secure: bool,
    /// Session cookie `SameSite` policy.
    pub cookie_samesite: SameSite,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            password: None,
            allowed_origins: default_origins(),
            cookie_secure: false,
            cookie_samesite: SameSite::Lax,
        }
    }
}

impl AuthConfig {
    /// Build authentication settings for tests or callers with a known PIN.
    #[must_use]
    pub fn with_password(password: impl Into<String>) -> Self {
        Self {
            password: Some(password.into()),
            ..Self::default()
        }
    }

    /// Resolve authentication settings from the server configuration.
    pub fn from_server_config(config: &ServerConfig) -> Result<Self, ServerError> {
        let password = config
            .password
            .as_deref()
            .map(str::trim)
            .filter(|pin| !pin.is_empty())
            .map(str::to_owned);

        let allowed_origins = match config.allowed_origins.as_deref() {
            Some(value) => parse_origins(value)?,
            None => default_origins(),
        };

        if config.cookie_samesite == SameSite::None && !config.cookie_secure {
            return Err(ServerError::InsecureCookie);
        }

        Ok(Self {
            password,
            allowed_origins,
            cookie_secure: config.cookie_secure,
            cookie_samesite: config.cookie_samesite,
        })
    }
}

fn default_origins() -> Vec<String> {
    DEFAULT_ORIGINS
        .iter()
        .map(|origin| (*origin).to_owned())
        .collect()
}

fn parse_origins(value: &str) -> Result<Vec<String>, ServerError> {
    let mut origins = Vec::new();

    for raw in value.split(',') {
        let origin = raw.trim().trim_end_matches('/');
        if origin.is_empty() {
            continue;
        }
        if !is_origin(origin) {
            return Err(ServerError::InvalidOrigin);
        }
        origins.push(origin.to_owned());
    }

    if origins.is_empty() {
        return Err(ServerError::InvalidOrigin);
    }

    Ok(origins)
}

fn is_origin(value: &str) -> bool {
    let Ok(uri) = value.parse::<axum::http::Uri>() else {
        return false;
    };

    matches!(uri.scheme_str(), Some("http" | "https"))
        && uri.authority().is_some()
        && matches!(uri.path(), "" | "/")
        && uri.query().is_none()
}

/// Why a login attempt was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoginError {
    /// The supplied PIN does not match.
    InvalidPin,
    /// Too many failures; retry after the reported delay.
    TooManyAttempts {
        /// How long the caller must wait.
        retry_after: Duration,
    },
}

/// Stateless authentication state shared by the HTTP and WebSocket handlers.
#[derive(Clone)]
pub struct Auth {
    inner: Arc<Inner>,
}

struct Inner {
    pin: ring::digest::Digest,
    key: hmac::Key,
    epoch: Instant,
    generated_pin: Option<String>,
    allowed_origins: Vec<String>,
    cookie_secure: bool,
    cookie_samesite: SameSite,
    limiter: Mutex<Limiter>,
}

impl Auth {
    /// Build authentication state, generating a PIN and signing key when needed.
    pub fn new(config: AuthConfig) -> Result<Self, ServerError> {
        let (pin, generated_pin) = match config.password {
            Some(pin) => (pin, None),
            None => {
                let pin = generate_pin()?;
                (pin.clone(), Some(pin))
            }
        };

        let mut key_bytes = [0u8; 32];
        SystemRandom::new()
            .fill(&mut key_bytes)
            .map_err(|_| ServerError::Random)?;

        Ok(Self {
            inner: Arc::new(Inner {
                pin: digest(&SHA256, pin.as_bytes()),
                key: hmac::Key::new(hmac::HMAC_SHA256, &key_bytes),
                epoch: Instant::now(),
                generated_pin,
                allowed_origins: config.allowed_origins,
                cookie_secure: config.cookie_secure,
                cookie_samesite: config.cookie_samesite,
                limiter: Mutex::new(Limiter::default()),
            }),
        })
    }

    /// The generated PIN, when `KIRA_PASSWORD` was not set.
    #[must_use]
    pub fn generated_pin(&self) -> Option<&str> {
        self.inner.generated_pin.as_deref()
    }

    /// Exact origins allowed to call the API.
    #[must_use]
    pub fn allowed_origins(&self) -> &[String] {
        &self.inner.allowed_origins
    }

    /// Whether an origin header value is allowed.
    #[must_use]
    pub fn origin_allowed(&self, origin: &str) -> bool {
        self.inner
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
    }

    /// Build the `Set-Cookie` header for a successful login.
    #[must_use]
    pub fn session_cookie(&self) -> HeaderValue {
        self.cookie(self.issue_token(), SESSION_TTL.as_secs())
    }

    /// Build the `Set-Cookie` header that clears the session cookie.
    #[must_use]
    pub fn clear_cookie(&self) -> HeaderValue {
        self.cookie(String::new(), 0)
    }

    /// Read the session token from a cookie header.
    #[must_use]
    pub fn token_from_headers(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .find_map(|pair| {
                let (name, value) = pair.trim().split_once('=')?;
                (name == COOKIE_NAME).then(|| value.to_owned())
            })
    }

    /// Validate a session token's signature and expiration.
    #[must_use]
    pub fn verify_token(&self, token: &str) -> bool {
        let mut parts = token.split('.');
        let (Some(TOKEN_VERSION), Some(exp), Some(tag)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return false;
        };
        if parts.next().is_some() {
            return false;
        }

        let Ok(exp) = exp.parse::<u64>() else {
            return false;
        };
        let Some(tag) = from_hex(tag) else {
            return false;
        };

        let payload = format!("{TOKEN_VERSION}.{exp}");
        hmac::verify(&self.inner.key, payload.as_bytes(), &tag).is_ok()
            && self.inner.epoch.elapsed().as_secs() < exp
    }

    /// Check a PIN, returning a session cookie header on success.
    pub fn login(&self, pin: &str, peer: Option<IpAddr>) -> Result<HeaderValue, LoginError> {
        let now = Instant::now();
        self.limiter().check(peer, now)?;

        let candidate = digest(&SHA256, pin.as_bytes());
        let matches: bool = candidate.as_ref().ct_eq(self.inner.pin.as_ref()).into();

        if matches {
            self.limiter().reset(peer);
            Ok(self.session_cookie())
        } else {
            self.limiter().record_failure(peer, now);
            Err(LoginError::InvalidPin)
        }
    }

    fn cookie(&self, value: String, max_age: u64) -> HeaderValue {
        let secure = if self.inner.cookie_secure {
            "; Secure"
        } else {
            ""
        };
        let cookie = format!(
            "{COOKIE_NAME}={value}; HttpOnly; Path=/; Max-Age={max_age}; SameSite={}{secure}",
            self.inner.cookie_samesite
        );

        HeaderValue::from_str(&cookie).expect("session cookie is ASCII")
    }

    fn issue_token(&self) -> String {
        let exp = self.inner.epoch.elapsed().as_secs() + SESSION_TTL.as_secs();
        let payload = format!("{TOKEN_VERSION}.{exp}");
        let tag = hmac::sign(&self.inner.key, payload.as_bytes());
        format!("{payload}.{}", to_hex(tag.as_ref()))
    }

    fn limiter(&self) -> MutexGuard<'_, Limiter> {
        self.inner
            .limiter
            .lock()
            .expect("login limiter mutex is not poisoned")
    }
}

fn generate_pin() -> Result<String, ServerError> {
    let rng = SystemRandom::new();

    loop {
        let mut bytes = [0u8; 4];
        rng.fill(&mut bytes).map_err(|_| ServerError::Random)?;
        let value = u32::from_be_bytes(bytes);
        // Largest multiple of 1_000_000 that fits in u32, for an unbiased draw.
        if value < 4_294_000_000 {
            return Ok(format!("{:06}", value % 1_000_000));
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn from_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }

    let (pairs, _) = value.as_bytes().as_chunks::<2>();
    let mut bytes = Vec::with_capacity(pairs.len());
    for [high_byte, low_byte] in pairs {
        let high = hex_digit(*high_byte)?;
        let low = hex_digit(*low_byte)?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Default)]
struct Limiter {
    peers: HashMap<IpAddr, Bucket>,
    unknown: Bucket,
    global: Bucket,
}

impl Limiter {
    fn check(&self, peer: Option<IpAddr>, now: Instant) -> Result<(), LoginError> {
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

    fn record_failure(&mut self, peer: Option<IpAddr>, now: Instant) {
        self.global.record_failure(now);

        let Some(peer) = peer else {
            self.unknown.record_failure(now);
            return;
        };

        if self.peers.len() >= MAX_TRACKED_PEERS {
            self.peers
                .retain(|_, bucket| bucket.retry_after(now).is_some());
        }

        self.peers.entry(peer).or_default().record_failure(now);
    }

    fn reset(&mut self, peer: Option<IpAddr>) {
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
    use super::*;

    fn auth() -> Auth {
        Auth::new(AuthConfig::with_password("123456")).expect("test auth")
    }

    #[test]
    fn tokens_round_trip() {
        let auth = auth();
        let cookie = auth.session_cookie().to_str().expect("cookie").to_owned();
        let token = cookie
            .trim_start_matches(&format!("{COOKIE_NAME}="))
            .split(';')
            .next()
            .expect("token");

        assert!(auth.verify_token(token));
    }

    #[test]
    fn tampered_tokens_are_rejected() {
        let auth = auth();
        let cookie = auth.session_cookie().to_str().expect("cookie").to_owned();
        let token = cookie
            .trim_start_matches(&format!("{COOKIE_NAME}="))
            .split(';')
            .next()
            .expect("token");

        let mut tampered = token.to_owned();
        tampered.push('0');
        assert!(!auth.verify_token(&tampered));
        assert!(!auth.verify_token("nonsense"));
    }

    #[test]
    fn hex_round_trips() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(from_hex(&to_hex(&bytes)), Some(bytes.to_vec()));
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
    }

    #[test]
    fn invalid_origins_are_rejected() {
        assert!(parse_origins("http://127.0.0.1:3000").is_ok());
        assert!(parse_origins("https://kira.example.com/").is_ok());
        assert!(parse_origins("*").is_err());
        assert!(parse_origins("http://").is_err());
        assert!(parse_origins("http://localhost:3000/path").is_err());
    }
}
