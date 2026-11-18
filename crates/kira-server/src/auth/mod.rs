//! PIN authentication, signed session cookies, and login rate limiting.

mod limiter;
mod origins;
mod token;

use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, header};
use kira_config::{SameSite, ServerConfig};
use kira_core::secret::SecretString;
use ring::digest::{SHA256, digest};
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use subtle::ConstantTimeEq;
use tokio::time::Instant;

use crate::errors::{LoginError, ServerError};

use limiter::Limiter;
use origins::{default_cli_bind, default_origins, parse_origins};
use token::{TOKEN_VERSION, from_hex, to_hex};

/// Session cookie name.
pub const COOKIE_NAME: &str = "kira_session";

/// Session lifetime.
pub const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Authentication settings for the server.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AuthConfig {
    /// Configured PIN or password. Generated when absent.
    pub password: Option<SecretString>,
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
            allowed_origins: default_origins(default_cli_bind()),
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
            password: Some(SecretString::new(password)),
            ..Self::default()
        }
    }

    /// Resolve authentication settings from the server configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the origin allowlist is malformed, a non-loopback
    /// dashboard has no explicit origins, or `SameSite=None` lacks `Secure`.
    pub fn from_server_config(config: &ServerConfig) -> Result<Self, ServerError> {
        let password = config
            .password
            .as_ref()
            .map(SecretString::expose)
            .map(str::trim)
            .filter(|pin| !pin.is_empty())
            .map(SecretString::new);

        let allowed_origins = match config.allowed_origins.as_deref() {
            Some(value) => parse_origins(value)?,
            None if config.cli_bind.ip().is_loopback() => default_origins(config.cli_bind),
            None => {
                return Err(ServerError::OriginsRequired {
                    addr: config.cli_bind,
                });
            }
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
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system cannot provide random bytes.
    pub fn new(config: AuthConfig) -> Result<Self, ServerError> {
        let (pin, generated_pin) = match config.password {
            Some(pin) => (digest(&SHA256, pin.expose().as_bytes()), None),
            None => {
                let pin = generate_pin()?;
                (digest(&SHA256, pin.as_bytes()), Some(pin))
            }
        };

        let mut key_bytes = [0u8; 32];
        SystemRandom::new()
            .fill(&mut key_bytes)
            .map_err(|_| ServerError::Random)?;

        Ok(Self {
            inner: Arc::new(Inner {
                pin,
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
    pub fn token_from_headers<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .find_map(|pair| {
                let (name, value) = pair.trim().split_once('=')?;
                (name == COOKIE_NAME).then_some(value)
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
    ///
    /// # Errors
    ///
    /// Returns [`LoginError::InvalidPin`] when the PIN does not match, or
    /// [`LoginError::TooManyAttempts`] while the caller is rate limited.
    pub fn login(&self, pin: &str, peer: Option<IpAddr>) -> Result<HeaderValue, LoginError> {
        let now = Instant::now();
        // Check and record under one lock so a concurrent burst cannot all
        // pass the limit before any failure is counted.
        let mut limiter = self.limiter();
        limiter.check(peer, now)?;

        let candidate = digest(&SHA256, pin.as_bytes());
        let matches: bool = candidate.as_ref().ct_eq(self.inner.pin.as_ref()).into();

        if matches {
            limiter.reset(peer);
            Ok(self.session_cookie())
        } else {
            limiter.record_failure(peer, now);
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
    fn generated_pin_logins_succeed() {
        let auth = Auth::new(AuthConfig::default()).expect("test auth");
        let pin = auth.generated_pin().expect("generated pin").to_owned();

        assert!(auth.login(&pin, None).is_ok());
        // A non-numeric PIN can never collide with the generated six-digit PIN.
        assert!(auth.login("definitely-not-the-pin", None).is_err());
    }

    #[test]
    fn non_loopback_dashboard_requires_explicit_origins() {
        let config = ServerConfig::try_from_iter([
            ("KIRA_GUILD_ID", "1"),
            ("KIRA_CLI_BIND", "0.0.0.0:3000"),
        ])
        .expect("config");

        assert!(matches!(
            AuthConfig::from_server_config(&config),
            Err(ServerError::OriginsRequired { .. })
        ));
    }
}
