//! Custom DNS resolution for Polymarket requests.
//!
//! # Why this exists
//!
//! `SPLIT_TUNNEL_IFACE` (see [`crate::api::http_client`]) binds sockets to an
//! interface, but name resolution still goes through the system resolver. When that
//! resolver is untrustworthy — an ISP resolver that answers Polymarket hostnames with
//! loopback, say — binding the socket cannot help: the connection is already aimed at
//! the wrong address.
//!
//! Setting `DNS_RESOLVER` makes Polymarket lookups skip the system resolver and query
//! the listed nameservers directly. Only the Polymarket clients use it, so the system
//! resolver still serves everything else (Binance, ESPN, and any LAN-local names that
//! only the local resolver knows about).
//!
//! # Configuration
//!
//! Set `DNS_RESOLVER` in the environment (see [`crate::config`]):
//!
//! ```text
//! DNS_RESOLVER=1.1.1.1          # single nameserver
//! DNS_RESOLVER=1.1.1.1,9.9.9.9  # tried in order
//! ```
//!
//! Unset (the default) keeps the system resolver.
//!
//! Queries are plain UDP/TCP on port 53, so a resolver reachable only over the split
//! tunnel requires routing for it. A network that tampers with port 53 itself would
//! need DNS-over-TLS, which this module does not yet configure.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, OnceLock};

use hickory_resolver::config::{LookupIpStrategy, NameServerConfig, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::net::NetError;
use hickory_resolver::TokioResolver;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::config::get_config;

/// A resolver that queries an explicit set of nameservers.
#[derive(Debug, Clone)]
pub struct ConfiguredResolver {
    resolver: TokioResolver,
}

impl ConfiguredResolver {
    /// Builds a resolver querying `nameservers` over UDP with TCP fallback.
    ///
    /// Resolution is IPv4-only: the split tunnel carries no IPv6, so an AAAA answer
    /// would either bypass the tunnel or be unroutable.
    fn new(nameservers: &[IpAddr]) -> Result<Self, NetError> {
        let servers = nameservers
            .iter()
            .copied()
            .map(NameServerConfig::udp_and_tcp)
            .collect();

        let config = ResolverConfig::from_parts(None, vec![], servers);
        let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
        builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4Only;

        Ok(Self {
            resolver: builder.build()?,
        })
    }
}

impl Resolve for ConfiguredResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.resolver.clone();
        Box::pin(async move {
            let lookup = resolver.lookup_ip(name.as_str()).await?;
            // Port 0: reqwest substitutes the scheme's port, or the URL's if given.
            let addrs: Addrs = Box::new(
                lookup
                    .iter()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|ip| SocketAddr::new(ip, 0)),
            );
            Ok(addrs)
        })
    }
}

/// Returns the configured resolver, or `None` to use the system resolver.
///
/// Built once per process on first use.
pub fn configured_resolver() -> Option<Arc<ConfiguredResolver>> {
    static RESOLVER: OnceLock<Option<Arc<ConfiguredResolver>>> = OnceLock::new();

    RESOLVER
        .get_or_init(|| {
            let nameservers = &get_config().dns_resolver;
            if nameservers.is_empty() {
                return None;
            }

            match ConfiguredResolver::new(nameservers) {
                Ok(resolver) => {
                    log::debug!(
                        "Resolving Polymarket hostnames via {nameservers:?} (IPv4 only), bypassing the system resolver"
                    );
                    Some(Arc::new(resolver))
                }
                Err(err) => {
                    log::error!(
                        "Failed to build resolver for {nameservers:?}; using system resolver: {err}"
                    );
                    None
                }
            }
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builds_resolver_from_addresses() {
        let resolver = ConfiguredResolver::new(&["1.1.1.1".parse().unwrap()])
            .expect("resolver builds from an explicit nameserver");
        // Construction must not require network access.
        let _ = resolver;
    }
}
