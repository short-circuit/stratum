//! Endpoint URL validation for SSRF prevention.
//!
//! Stratum supports custom AI provider endpoints. This module validates
//! those URLs before any HTTP request is made, blocking SSRF attacks
//! against internal services. Only `https://` URLs and `http://` URLs
//! pointing to local/private network addresses are allowed.

use crate::PkmError;

/// Validate that `url` is safe to make HTTP requests to.
///
/// Validate that `url` is safe to make HTTP requests to.
///
/// # Rules
///
/// * `https://` — always allowed (TLS provides server identity).
/// * `http://` — allowed **only** for:
///   - `localhost`, `127.0.0.1`, `::1`
///   - Private IPv4 ranges: `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`
///   - `.local` and `.localdomain` suffixes
/// * All other schemes (e.g. `file://`, `ftp://`) are rejected.
/// * A malformed or unparseable URL is rejected.
///
/// # Errors
///
/// Returns `PkmError::Validation` with a human-readable message when the
/// URL is not safe to call.
pub fn validate_endpoint_safe(url: &str) -> Result<(), PkmError> {
    let parsed =
        url::Url::parse(url).map_err(|e| PkmError::Validation(format!("Invalid URL: {e}")))?;

    match parsed.scheme() {
        "https" => Ok(()),
        "http" => {
            match parsed.host() {
                // IPv4: check loopback or private range
                Some(url::Host::Ipv4(ip)) => {
                    if ip.is_loopback() || ip.is_private() {
                        Ok(())
                    } else {
                        Err(PkmError::Validation(format!(
                            "HTTP URL '{url}' is not allowed — only local/private hosts are \
                             permitted for plain HTTP. Use HTTPS for external endpoints."
                        )))
                    }
                }
                // IPv6: check loopback
                Some(url::Host::Ipv6(ip)) => {
                    if ip.is_loopback() {
                        Ok(())
                    } else {
                        Err(PkmError::Validation(format!(
                            "HTTP URL '{url}' is not allowed — only local/private hosts are \
                             permitted for plain HTTP. Use HTTPS for external endpoints."
                        )))
                    }
                }
                // Domain name: check localhost / .local / .localdomain
                Some(url::Host::Domain(domain)) => {
                    if domain.eq_ignore_ascii_case("localhost")
                        || domain == "local"
                        || domain == "localdomain"
                        || domain.ends_with(".local")
                        || domain.ends_with(".localdomain")
                    {
                        Ok(())
                    } else {
                        Err(PkmError::Validation(format!(
                            "HTTP URL '{url}' is not allowed — only local/private hosts are \
                             permitted for plain HTTP. Use HTTPS for external endpoints."
                        )))
                    }
                }
                None => Err(PkmError::Validation("URL has no host".to_string())),
            }
        }
        _ => Err(PkmError::Validation(format!(
            "Scheme '{}' is not allowed — only http and https are supported",
            parsed.scheme()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- HTTPS (always allowed) ---

    #[test]
    fn test_https_external_allowed() {
        assert!(validate_endpoint_safe("https://api.openai.com/v1").is_ok());
    }

    #[test]
    fn test_https_localhost_allowed() {
        assert!(validate_endpoint_safe("https://localhost:11434").is_ok());
    }

    #[test]
    fn test_https_private_ip_allowed() {
        assert!(validate_endpoint_safe("https://192.168.1.1:8080").is_ok());
    }

    // --- HTTP to localhost / loopback ---

    #[test]
    fn test_http_localhost_allowed() {
        assert!(validate_endpoint_safe("http://localhost:11434").is_ok());
    }

    #[test]
    fn test_http_127_0_0_1_allowed() {
        assert!(validate_endpoint_safe("http://127.0.0.1:11434").is_ok());
    }

    #[test]
    fn test_http_ipv6_localhost_allowed() {
        assert!(validate_endpoint_safe("http://[::1]:11434").is_ok());
    }

    // --- HTTP to private ranges ---

    #[test]
    fn test_http_10_x_allowed() {
        assert!(validate_endpoint_safe("http://10.0.0.5:8080").is_ok());
        assert!(validate_endpoint_safe("http://10.255.255.255").is_ok());
    }

    #[test]
    fn test_http_172_16_31_allowed() {
        assert!(validate_endpoint_safe("http://172.16.0.1").is_ok());
        assert!(validate_endpoint_safe("http://172.31.255.255").is_ok());
    }

    #[test]
    fn test_http_192_168_allowed() {
        assert!(validate_endpoint_safe("http://192.168.1.1").is_ok());
        assert!(validate_endpoint_safe("http://192.168.0.0").is_ok());
    }

    // --- HTTP .local / .localdomain ---

    #[test]
    fn test_http_local_suffix_allowed() {
        assert!(validate_endpoint_safe("http://my-ollama.local:11434").is_ok());
    }

    #[test]
    fn test_http_localdomain_suffix_allowed() {
        assert!(validate_endpoint_safe("http://server.localdomain").is_ok());
    }

    // --- HTTP to public hosts (blocked) ---

    #[test]
    fn test_http_public_host_rejected() {
        let result = validate_endpoint_safe("http://api.openai.com/v1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not allowed"));
    }

    #[test]
    fn test_http_public_ip_rejected() {
        let result = validate_endpoint_safe("http://8.8.8.8");
        assert!(result.is_err());
    }

    #[test]
    fn test_http_public_172_32_rejected() {
        // 172.32.x.x is outside the private 172.16-31 range
        let result = validate_endpoint_safe("http://172.32.0.1");
        assert!(result.is_err());
    }

    // --- Bad schemes ---

    #[test]
    fn test_ftp_rejected() {
        let result = validate_endpoint_safe("ftp://localhost");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Scheme"));
    }

    #[test]
    fn test_file_rejected() {
        let result = validate_endpoint_safe("file:///etc/passwd");
        assert!(result.is_err());
    }

    // --- Malformed URLs ---

    #[test]
    fn test_invalid_url_rejected() {
        let result = validate_endpoint_safe("not a url");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid URL"));
    }

    #[test]
    fn test_empty_url_rejected() {
        let result = validate_endpoint_safe("");
        assert!(result.is_err());
    }

    // --- Edge cases ---

    #[test]
    fn test_trailing_slash_ok() {
        assert!(validate_endpoint_safe("https://api.openai.com/").is_ok());
    }

    #[test]
    fn test_http_just_host_allowed_localhost() {
        assert!(validate_endpoint_safe("http://localhost").is_ok());
    }

    #[test]
    fn test_http_with_auth_localhost() {
        // user:password@localhost is still local
        assert!(validate_endpoint_safe("http://user:pass@localhost:11434").is_ok());
    }
}
