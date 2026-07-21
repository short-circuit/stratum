use crate::PkmResult;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use url::Host;

/// Validates that a URL points to a safe (non-private) endpoint.
///
/// Blocks SSRF attacks by rejecting URLs that resolve to:
/// - Loopback addresses (127.0.0.0/8, ::1)
/// - Private IPv4 ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
/// - Link-local addresses (169.254.0.0/16, including 169.254.169.254)
/// - CGNAT range (100.64.0.0/10)
/// - Current network (0.0.0.0/8)
/// - Benchmarking (198.18.0.0/15)
/// - IPv6 link-local (fe80::/10) and unique-local (fc00::/7)
///
/// Returns `Ok(())` if the URL is safe, or a `PkmError::Validation` error.
pub fn validate_endpoint_safe(url_str: &str) -> PkmResult<()> {
    let url = url::Url::parse(url_str)
        .map_err(|e| crate::PkmError::Validation(format!("Invalid URL: {e}")))?;

    let host = url
        .host()
        .ok_or_else(|| crate::PkmError::Validation("URL has no host".to_string()))?;

    match host {
        // If host is a parsed IP literal, check it directly.
        Host::Ipv4(ip) => {
            if is_private_ipv4(&ip) {
                return Err(crate::PkmError::Validation(format!(
                    "SSRF blocked: URL resolves to a private/reserved IP ({ip})"
                )));
            }
        }
        Host::Ipv6(ip) => {
            if is_private_ipv6(&ip) {
                return Err(crate::PkmError::Validation(format!(
                    "SSRF blocked: URL resolves to a private/reserved IP ({ip})"
                )));
            }
        }
        // For domain names, resolve to IP addresses and check each one.
        Host::Domain(domain) => {
            let addrs = (domain, 0u16).to_socket_addrs().map_err(|e| {
                crate::PkmError::Validation(format!("Failed to resolve host {domain}: {e}"))
            })?;

            for addr in addrs {
                if is_private_ip(&addr.ip()) {
                    return Err(crate::PkmError::Validation(format!(
                        "SSRF blocked: URL resolves to a private/reserved IP ({})",
                        addr.ip()
                    )));
                }
            }
        }
    }

    Ok(())
}

/// Returns `true` if the IP address is in a private or reserved range.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    // 0.0.0.0/8 — current network
    if octets[0] == 0 {
        return true;
    }
    // 10.0.0.0/8 — private
    if octets[0] == 10 {
        return true;
    }
    // 100.64.0.0/10 — CGNAT
    if octets[0] == 100 && (octets[1] & 0xC0) == 64 {
        return true;
    }
    // 127.0.0.0/8 — loopback
    if octets[0] == 127 {
        return true;
    }
    // 169.254.0.0/16 — link-local (includes 169.254.169.254)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    // 172.16.0.0/12 — private
    if octets[0] == 172 && (octets[1] & 0xF0) == 16 {
        return true;
    }
    // 192.168.0.0/16 — private
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    // 198.18.0.0/15 — benchmarking
    if octets[0] == 198 && (octets[1] & 0xFE) == 18 {
        return true;
    }
    false
}

fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    let octets = ip.octets();
    // ::1 — loopback
    if *ip == Ipv6Addr::LOCALHOST {
        return true;
    }
    // ::/128 — unspecified
    if ip.is_unspecified() {
        return true;
    }
    // fe80::/10 — link-local
    if octets[0] == 0xFE && (octets[1] & 0xC0) == 0x80 {
        return true;
    }
    // fc00::/7 — unique-local
    if (octets[0] & 0xFE) == 0xFC {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_public_ipv4() {
        assert!(validate_endpoint_safe("https://93.184.216.34/path").is_ok());
    }

    #[test]
    fn test_accepts_public_hostname() {
        assert!(validate_endpoint_safe("https://example.com/path").is_ok());
    }

    #[test]
    fn test_blocks_loopback_ipv4() {
        let err = validate_endpoint_safe("http://127.0.0.1:8080/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_blocks_loopback_ipv6() {
        let err = validate_endpoint_safe("http://[::1]:11434/api/chat").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_blocks_private_10() {
        let err = validate_endpoint_safe("http://10.0.0.5/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_blocks_private_172() {
        let err = validate_endpoint_safe("http://172.16.0.1/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_blocks_private_192() {
        let err = validate_endpoint_safe("http://192.168.1.1/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_blocks_metadata_169() {
        let err = validate_endpoint_safe("http://169.254.169.254/latest/meta-data/").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }

    #[test]
    fn test_rejects_invalid_url() {
        let err = validate_endpoint_safe("not-a-url").unwrap_err();
        assert!(err.to_string().contains("Invalid URL"));
    }

    #[test]
    fn test_rejects_no_host() {
        // A URL without a host component (e.g., just a scheme)
        let err = validate_endpoint_safe("file:///etc/passwd").unwrap_err();
        assert!(err.to_string().contains("no host"));
    }

    #[test]
    fn test_blocks_cgnat() {
        let err = validate_endpoint_safe("http://100.64.0.1/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));

        // 100.127.255.254 is also CGNAT
        let err = validate_endpoint_safe("http://100.127.255.254/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));

        // 100.128.0.1 is NOT CGNAT (public)
        assert!(validate_endpoint_safe("http://100.128.0.1/api").is_ok());
    }

    #[test]
    fn test_blocks_benchmarking() {
        let err = validate_endpoint_safe("http://198.18.0.1/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));

        let err = validate_endpoint_safe("http://198.19.255.255/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));

        // 198.20.0.1 is public
        assert!(validate_endpoint_safe("http://198.20.0.1/api").is_ok());
    }

    #[test]
    fn test_blocks_unspecified() {
        let err = validate_endpoint_safe("http://0.0.0.0/api").unwrap_err();
        assert!(err.to_string().contains("SSRF blocked"));
    }
}
