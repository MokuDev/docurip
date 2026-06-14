use std::net::IpAddr;
use url::Url;

/// Check if a hostname is a known private/internal target.
///
/// This performs two levels of checks:
/// 1. **Fast path**: If the hostname is an IP literal, check if it's in a
///    private/reserved range directly (no DNS needed).
/// 2. **Pattern path**: Block known local hostnames (localhost, *.local, etc.)
/// 3. **DNS path**: Resolve the hostname and check if the resulting IPs are private.
pub fn is_private_target(url: &str) -> bool {
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return true,
    };
    let host = match parsed.host_str() {
        Some(h) => h,
        None => return true,
    };

    if is_private_ip_literal(host) {
        return true;
    }
    if is_local_hostname(host) {
        return true;
    }
    if resolves_to_private_ip(host) {
        return true;
    }
    false
}

/// Check if a hostname string is an IP literal in a private/reserved range.
fn is_private_ip_literal(host: &str) -> bool {
    let addr: IpAddr = match host.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    is_private_addr(&addr)
}

fn is_private_addr(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.x.x.x
                || v4.is_unspecified() // 0.x.x.x
                || v4.is_link_local()  // 169.254.x.x
                || v4.is_private()     // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()    // ::1
                || v6.is_unspecified() // ::
                || v6.is_unicast_link_local() // fe80::
                || is_ipv6_unique_local(v6) // fc00::/7
        }
    }
}

/// Check fc00::/7 range (unique local addresses).
fn is_ipv6_unique_local(addr: &std::net::Ipv6Addr) -> bool {
    let octets = addr.octets();
    (octets[0] & 0xfe) == 0xfc
}

/// Block known local hostnames that don't look like IP literals.
fn is_local_hostname(host: &str) -> bool {
    let lower = host.to_lowercase();
    lower == "localhost"
        || lower.ends_with(".local")
        || lower.ends_with(".localhost")
        || lower == "0.0.0.0"
        || lower == "::"
        || lower == "[::1]"
        || lower == "broadcasthost"
        || lower == "ip6-localhost"
        || lower == "ip6-loopback"
}

/// Try DNS resolution and check if any result is private.
/// Returns false if DNS fails (don't block on resolution errors).
fn resolves_to_private_ip(host: &str) -> bool {
    use std::net::ToSocketAddrs;
    let socket_addr = format!("{}:80", host);
    match socket_addr.to_socket_addrs() {
        Ok(mut addrs) => addrs.any(|a| is_private_addr(&a.ip())),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_localhost() {
        assert!(is_private_target("http://localhost/page"));
        assert!(is_private_target("http://localhost:3000/api"));
    }

    #[test]
    fn blocks_loopback_ip() {
        assert!(is_private_target("http://127.0.0.1/secret"));
        assert!(is_private_target("http://127.0.0.1:8080/admin"));
    }

    #[test]
    fn blocks_private_cidrs() {
        assert!(is_private_target("http://10.0.0.1/"));
        assert!(is_private_target("http://10.255.255.255/"));
        assert!(is_private_target("http://172.16.0.1/"));
        assert!(is_private_target("http://172.31.255.255/"));
        assert!(is_private_target("http://192.168.1.1/"));
        assert!(is_private_target("http://192.168.0.1/"));
    }

    #[test]
    fn blocks_link_local() {
        assert!(is_private_target("http://169.254.1.1/"));
    }

    #[test]
    fn blocks_unspecified() {
        assert!(is_private_target("http://0.0.0.0/"));
    }

    #[test]
    fn blocks_ipv6_loopback() {
        assert!(is_private_target("http://[::1]/"));
    }

    #[test]
    fn blocks_local_tld() {
        assert!(is_private_target("http://myhost.local/"));
        assert!(is_private_target("http://service.local:8080/"));
    }

    #[test]
    fn allows_public_hosts() {
        assert!(!is_private_target("https://example.com"));
        assert!(!is_private_target("https://docs.example.org/intro"));
        assert!(!is_private_target("http://8.8.8.8/"));
    }

    #[test]
    fn resolves_dns_for_private() {
        assert!(is_private_target("http://localhost:3000/"));
    }

    #[test]
    fn invalid_url_treated_as_private() {
        assert!(is_private_target("not-a-url"));
        assert!(is_private_target("file:///etc/passwd"));
    }
}
