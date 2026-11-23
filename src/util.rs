use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Public mode is opt-in. It is used by the Docker/Hugging Face image and
/// changes both the bind address and the crawler's SSRF policy.
pub fn public_mode() -> bool {
    env_flag("GEOSIGNAL_PUBLIC")
}

fn private_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
        || octets[0] == 0
        || octets[0] >= 240
        || (octets[0] == 100 && (64..128).contains(&octets[1]))
        || (octets[0] == 198 && (18..20).contains(&octets[1]))
}

fn private_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return private_ipv4(mapped);
    }
    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
}

fn private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => private_ipv4(ip),
        IpAddr::V6(ip) => private_ipv6(ip),
    }
}

/// Reject single-label, local and private targets when the app is exposed.
/// The DNS check catches names that resolve to a private address; redirects
/// are checked again by the crawler.
pub fn is_private_host(host: &str) -> bool {
    let host = host
        .trim()
        .trim_matches('[')
        .trim_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host == "metadata.google.internal"
    {
        return true;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return private_ip(ip);
    }
    if !host.contains('.') {
        return true;
    }
    match (host.as_str(), 443).to_socket_addrs() {
        Ok(addresses) => addresses.map(|address| address.ip()).any(private_ip),
        Err(_) => false,
    }
}

pub fn public_url_allowed(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    !is_private_host(host)
}

/// Connect-time SSRF guard for public deployments, used as ureq's resolver.
/// The URL check runs first, but a hostname can resolve again to a different
/// address before the connection is made. Filtering here means the address the
/// socket actually uses has been checked, which closes that rebinding window.
pub fn public_resolver(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let addresses: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
    if addresses.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "host has no addresses",
        ));
    }
    let allowed: Vec<SocketAddr> = addresses
        .into_iter()
        .filter(|address| !private_ip(address.ip()))
        .collect();
    if allowed.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "public deployments only connect to public addresses",
        ));
    }
    Ok(allowed)
}

pub fn data_dir() -> PathBuf {
    std::env::var("GEOSIGNAL_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"))
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

pub fn now_iso() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        duration.subsec_millis()
    )
}

pub fn new_id(prefix: &str) -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}_{:x}{:x}",
        prefix,
        duration.as_millis(),
        duration.subsec_nanos()
    )
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn pct(part: usize, total: usize) -> usize {
    if total == 0 {
        0
    } else {
        (part * 100 + total / 2) / total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_and_loopback_hosts() {
        for host in [
            "localhost",
            "127.0.0.1",
            "10.0.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "::1",
            "[::1]",
            "fd00::1",
            "metadata.google.internal",
            "service.local",
        ] {
            assert!(is_private_host(host), "{host}");
        }
        assert!(!is_private_host("8.8.8.8"));
        assert!(!public_url_allowed("http://127.0.0.1:8080/"));
        assert!(!public_url_allowed("file:///etc/passwd"));
    }

    #[test]
    fn connect_time_resolver_blocks_private_addresses() {
        assert!(public_resolver("127.0.0.1:80").is_err());
        assert!(public_resolver("169.254.169.254:80").is_err());
        assert!(public_resolver("[::1]:443").is_err());
        assert!(public_resolver("10.0.0.5:8080").is_err());
    }
}
