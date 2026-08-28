//! Configuration for this crate, supplied by the caller.
//!
//! This library never loads `.env` and never reads the process environment on its own:
//! where values come from is the caller's decision. The application builds a [`Config`]
//! — from `.env`, a config file, a secrets manager, or literals in a test — and installs
//! it once via [`init`]. Everything in this crate then reads it through [`get_config`].
//!
//! # Usage
//!
//! ```no_run
//! use poly_clob_rs::config::{self, Config};
//!
//! fn main() {
//!     dotenvy::dotenv().ok();               // the caller decides to use .env
//!     config::init(Config::from_env());     // ... and installs the result
//! }
//! ```
//!
//! [`Config::from_env`] is a convenience for the common case; construct the struct
//! directly to source values from anywhere else.
//!
//! # Why a singleton
//!
//! HTTP clients and the DNS resolver are process-global and built lazily on first use.
//! Reading the environment at that moment makes behaviour depend on *when* the first
//! request happens: a client built before `.env` was loaded caches the wrong answer for
//! the process lifetime — which is exactly how a `DNS_RESOLVER` override once silently
//! did nothing. Installing an immutable snapshot up front removes that class of bug.
//!
//! For the same reason [`get_config`] panics when [`init`] has not been called, rather
//! than quietly falling back to defaults.

use std::net::IpAddr;
use std::sync::OnceLock;

use crate::api::error::{AuthError, Result};
use crate::api::relayer::SignatureType;

/// Interface name for split-tunnelling Polymarket traffic. See [`crate::api::http_client`].
pub const SPLIT_TUNNEL_IFACE_ENV: &str = "SPLIT_TUNNEL_IFACE";

/// Nameservers for resolving Polymarket hostnames. See [`crate::api::dns`].
pub const DNS_RESOLVER_ENV: &str = "DNS_RESOLVER";

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Installs the process-wide configuration.
///
/// Call once, early in `main`, before any client or resolver is built. Later calls are
/// ignored: the first install wins, so a stray call cannot swap configuration out from
/// under clients already built against it.
///
/// Returns whether this call performed the install, for callers that care.
pub fn init(config: Config) -> bool {
    CONFIG.set(config).is_ok()
}

/// Loads `.env`, then installs [`Config::from_env`].
///
/// A convenience for callers whose configuration lives in `.env` — tests, examples, and
/// small binaries. The library never calls this itself: opting into `.env` stays the
/// caller's decision. Applications in this workspace go through
/// `core_services::controller::config::init_from_env`, which also installs the
/// application config.
///
/// Returns whether this call performed the install, for callers that care.
#[allow(
    clippy::must_use_candidate,
    reason = "called for its effect on the config singleton; the install flag is incidental, and \
              sibling `init` is unmarked for the same reason"
)]
pub fn init_from_env() -> bool {
    dotenvy::dotenv().ok();
    init(Config::from_env())
}

/// Returns the installed configuration.
///
/// # Panics
///
/// Panics if [`init`] has not been called. This is deliberate: silently defaulting
/// would mean credentials, split tunnelling and DNS overrides quietly doing nothing.
pub fn get_config() -> &'static Config {
    CONFIG.get().expect(
        "poly_clob_rs::config::init() must be called before use \
         (e.g. `config::init(Config::from_env())` at the top of main)",
    )
}

/// Returns the installed configuration, or `None` when [`init`] has not been called.
///
/// For callers that must not panic; prefer [`get_config`].
pub fn try_get_config() -> Option<&'static Config> {
    CONFIG.get()
}

/// Everything this crate needs from its host application.
///
/// Credentials are optional: paper- and replay-only runs have none, and that is only an
/// error where one is actually required — see [`Config::require`].
#[derive(Debug, Clone, Default)]
pub struct Config {
    // --- Polymarket account ---
    pub poly_address: Option<String>,
    pub pub_key: Option<String>,
    pub private_key: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
    pub account_name: Option<String>,
    /// Wallet type; [`SignatureType::PolyProxy`] by default.
    pub signature_type: SignatureType,

    // --- Builder API (relayer) ---
    pub poly_builder_api_key: Option<String>,
    pub poly_builder_api_secret: Option<String>,
    pub poly_builder_api_passphrase: Option<String>,

    // --- Telegram (carried on `Account`) ---
    /// `None` disables Telegram; [`Config::from_env`] maps a `0` chat id to `None`.
    pub telegram_chat_id: Option<i64>,
    pub telegram_bot_token: Option<String>,

    // --- Network policy ---
    /// Interface to bind Polymarket sockets to; `None` uses default system routing.
    pub split_tunnel_iface: Option<String>,
    /// Nameservers for Polymarket hostnames; empty uses the system resolver.
    pub dns_resolver: Vec<IpAddr>,
}

impl Config {
    /// Builds a `Config` from the process environment.
    ///
    /// Does **not** load `.env` — call `dotenvy::dotenv()` first if that is where the
    /// values live. Unset, empty and whitespace-only values all read as absent, so a
    /// blank entry means "not configured" rather than an empty credential.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            poly_address: var("POLY_ADDRESS"),
            pub_key: var("PUB_KEY"),
            private_key: var("PRIVATE_KEY"),
            api_key: var("API_KEY"),
            api_secret: var("API_SECRET"),
            api_passphrase: var("API_PASSPHRASE"),
            account_name: var("ACCOUNT_NAME"),
            signature_type: var("SIGNATURE_TYPE")
                .and_then(|s| s.parse().ok())
                .unwrap_or(SignatureType::PolyProxy),

            poly_builder_api_key: var("POLY_BUILDER_API_KEY"),
            poly_builder_api_secret: var("POLY_BUILDER_API_SECRET"),
            poly_builder_api_passphrase: var("POLY_BUILDER_API_PASSPHRASE"),

            telegram_chat_id: var("TELEGRAM_CHAT_ID")
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&id| id != 0),
            telegram_bot_token: var("TELEGRAM_BOT_TOKEN"),

            split_tunnel_iface: var(SPLIT_TUNNEL_IFACE_ENV),
            dns_resolver: parse_nameservers(var(DNS_RESOLVER_ENV)),
        }
    }

    /// Returns `field`, or an error naming the missing setting.
    ///
    /// `var_name` is the environment variable the value conventionally comes from, used
    /// only to make the error actionable.
    ///
    /// # Errors
    ///
    /// [`AuthError::MissingEnvVar`] naming `var_name`, if `field` is `None`.
    pub fn require(&self, field: &Option<String>, var_name: &str) -> Result<String> {
        field.clone().ok_or_else(|| {
            AuthError::MissingEnvVar {
                var_name: var_name.to_string(),
            }
            .into()
        })
    }
}

/// Reads a variable, treating unset, empty and whitespace-only as absent.
fn var(key: &str) -> Option<String> {
    let value = std::env::var(key).ok()?;
    let trimmed = value.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parses a comma-separated nameserver list, warning on and skipping bad entries.
fn parse_nameservers(raw: Option<String>) -> Vec<IpAddr> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let ips: Vec<IpAddr> = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| match entry.parse::<IpAddr>() {
            Ok(ip) => Some(ip),
            Err(_) => {
                log::warn!("{DNS_RESOLVER_ENV}: ignoring unparseable nameserver '{entry}'");
                None
            }
        })
        .collect();

    if ips.is_empty() {
        log::warn!("{DNS_RESOLVER_ENV} is set but held no usable nameserver; using system resolver");
    }

    ips
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nameserver_lists() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();

        assert_eq!(parse_nameservers(Some("1.1.1.1".into())), vec![ip("1.1.1.1")]);
        assert_eq!(
            parse_nameservers(Some(" 1.1.1.1 , 9.9.9.9 ".into())),
            vec![ip("1.1.1.1"), ip("9.9.9.9")]
        );
        assert_eq!(parse_nameservers(Some("bogus,1.1.1.1".into())), vec![ip("1.1.1.1")]);
        assert!(parse_nameservers(Some("not-an-ip".into())).is_empty());
        assert!(parse_nameservers(None).is_empty());
    }

    #[test]
    fn blank_values_read_as_absent() {
        // SAFETY: keys are unique to this test and not read concurrently.
        unsafe {
            std::env::set_var("POLY_CONFIG_TEST_BLANK", "   ");
            std::env::set_var("POLY_CONFIG_TEST_SET", "  value  ");
        }

        assert_eq!(var("POLY_CONFIG_TEST_BLANK"), None);
        assert_eq!(var("POLY_CONFIG_TEST_SET"), Some("value".to_string()));
        assert_eq!(var("POLY_CONFIG_TEST_MISSING"), None);
    }

    #[test]
    fn require_names_the_missing_variable() {
        let config = Config::default();
        let err = config
            .require(&config.poly_address, "POLY_ADDRESS")
            .unwrap_err();

        assert!(err.to_string().contains("POLY_ADDRESS"), "got: {err}");
    }
}
