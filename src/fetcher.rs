use placard_render::Fetcher;
use std::fmt;
use std::net::IpAddr;
use std::time::Duration;
use ureq::Agent;
use ureq::config::Config;
use ureq::http::Uri;
use ureq::tls::{TlsConfig, TlsProvider};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver as UreqResolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_SIZE: u64 = 512 * 1024;

pub struct UreqFetcher {
    agent: Agent,
}

impl UreqFetcher {
    pub fn new() -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(FETCH_TIMEOUT))
            .tls_config(TlsConfig::builder().provider(TlsProvider::Rustls).build())
            .build();
        let agent = Agent::with_parts(
            config,
            DefaultConnector::default(),
            SsrfGuardedResolver::default(),
        );
        Self { agent }
    }
}

impl Fetcher for UreqFetcher {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut response = self.agent.get(url).call().map_err(|e| e.to_string())?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_SIZE)
            .read_to_vec()
            .map_err(|e| e.to_string())
    }
}

/// Wraps ureq's normal DNS resolution and rejects any candidate address
/// that isn't globally reachable, before ureq ever opens a connection.
/// This has to happen at the resolver step rather than by validating a
/// URL's hostname up front: validating a hostname and then letting ureq
/// re-resolve it independently would leave a DNS-rebinding gap (a
/// malicious name server could hand back a safe address for the check and
/// a private one moments later, for the actual connection).
#[derive(Default)]
struct SsrfGuardedResolver {
    inner: DefaultResolver,
}

impl fmt::Debug for SsrfGuardedResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SsrfGuardedResolver").finish()
    }
}

impl UreqResolver for SsrfGuardedResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let candidates = self.inner.resolve(uri, config, timeout)?;
        let mut safe = self.empty();
        for addr in candidates.iter() {
            if is_globally_reachable(addr.ip()) {
                safe.push(*addr);
            }
        }
        if safe.is_empty() {
            return Err(ureq::Error::HostNotFound);
        }
        Ok(safe)
    }
}

fn is_globally_reachable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_globally_reachable(IpAddr::V4(mapped));
            }
            !(v6.is_loopback()
                || v6.is_multicast()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn rejects_private_and_loopback_and_link_local_v4() {
        assert!(!is_globally_reachable(IpAddr::V4(Ipv4Addr::new(
            10, 0, 0, 1
        ))));
        assert!(!is_globally_reachable(IpAddr::V4(Ipv4Addr::new(
            192, 168, 1, 1
        ))));
        assert!(!is_globally_reachable(IpAddr::V4(Ipv4Addr::new(
            172, 16, 0, 1
        ))));
        assert!(!is_globally_reachable(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(!is_globally_reachable(IpAddr::V4(Ipv4Addr::new(
            169, 254, 169, 254
        ))));
    }

    #[test]
    fn rejects_loopback_and_unique_local_v6() {
        assert!(!is_globally_reachable(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_globally_reachable(IpAddr::V6(Ipv6Addr::new(
            0xfc00, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(!is_globally_reachable(IpAddr::V6(Ipv6Addr::new(
            0xfe80, 0, 0, 0, 0, 0, 0, 1
        ))));
    }

    #[test]
    fn rejects_ipv4_mapped_private_addresses() {
        // ::ffff:10.0.0.1 -- must unwrap to the IPv4 rules, not slip through
        // as an unrecognized IPv6 address.
        let mapped = Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0001);
        assert!(!is_globally_reachable(IpAddr::V6(mapped)));
    }

    #[test]
    fn accepts_ordinary_public_addresses() {
        assert!(is_globally_reachable(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(is_globally_reachable(IpAddr::V6(Ipv6Addr::new(
            0x2001, 0x4860, 0, 0, 0, 0, 0, 0x8888
        ))));
    }
}
