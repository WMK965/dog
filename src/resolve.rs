//! Specifying the address of the DNS server to send requests to.

use std::fmt;
use std::io;

use log::*;

use dns::Labels;


/// A **resolver type** is the source of a `Resolver`.
#[derive(PartialEq, Debug)]
pub enum ResolverType {

    /// Obtain a resolver by consulting the system in order to find a
    /// nameserver and a search list.
    SystemDefault,

    /// Obtain a resolver by using the given user-submitted string.
    Specific(String),
}

impl ResolverType {

    /// Obtains a resolver by the means specified in this type. Returns an
    /// error if there was a problem looking up system information, or if
    /// there is no suitable nameserver available.
    pub fn obtain(self) -> Result<Resolver, ResolverLookupError> {
        match self {
            Self::SystemDefault => {
                system_nameservers()
            }
            Self::Specific(nameserver) => {
                let search_list = Vec::new();
                Ok(Resolver { nameserver, search_list })
            }
        }
    }
}


/// A **resolver** knows the address of the server we should
/// send DNS requests to, and the search list for name lookup.
#[derive(Debug)]
pub struct Resolver {

    /// The address of the nameserver.
    pub nameserver: String,

    /// The search list for name lookup.
    pub search_list: Vec<String>,
}

impl Resolver {

    /// Returns a nameserver that queries should be sent to.
    pub fn nameserver(&self) -> String {
        self.nameserver.clone()
    }

    /// Returns a sequence of names to be queried, taking into account
    /// the search list.
    pub fn name_list(&self, name: &Labels) -> Vec<Labels> {
        let mut list = Vec::new();

        if name.len() > 1 {
            list.push(name.clone());
            return list;
        }

        for search in &self.search_list {
            match Labels::encode(search) {
                Ok(suffix)  => list.push(name.extend(&suffix)),
                Err(_)      => warn!("Invalid search list: {}", search),
            }
        }

        list.push(name.clone());
        list
    }
}


/// Looks up the system default nameserver on Unix, using a multi-tier
/// fallback strategy:
///
/// 1. `/etc/resolv.conf` (traditional)
/// 2. `resolvectl dns` — link DNS first (physical iface), global DNS second
/// 3. `/run/systemd/resolve/resolv.conf` (works without resolvectl binary)
/// 4. `scutil --dns` (macOS)
/// 5. `1.1.1.1` (last resort)
#[cfg(unix)]
fn system_nameservers() -> Result<Resolver, ResolverLookupError> {
    if cfg!(test) {
        panic!("system_nameservers() called from test code");
    }

    let search_list = Vec::new();

    let nameserver = read_resolv_conf().ok().and_then(|(ns, _sl)| {
        crate::verbose!("[dog]   Found nameserver in /etc/resolv.conf: {}", ns);
        Some(ns)
    });

    let nameserver = nameserver.or_else(|| {
        crate::verbose!("[dog] Trying resolvectl (systemd-resolved)...");
        resolvectl_dns().map(|ns| {
            crate::verbose!("[dog]   Found nameserver via resolvectl: {}", ns);
            ns
        })
    });

    let nameserver = nameserver.or_else(|| {
        crate::verbose!("[dog] Trying /run/systemd/resolve/resolv.conf...");
        read_resolv_conf_at("/run/systemd/resolve/resolv.conf")
            .or_else(|_| read_resolv_conf_at("/run/systemd/resolve/stub-resolv.conf"))
            .ok()
            .map(|(ns, _sl)| {
                crate::verbose!("[dog]   Found nameserver in systemd-resolve config: {}", ns);
                ns
            })
    });

    let nameserver = nameserver.or_else(|| {
        crate::verbose!("[dog] Trying scutil --dns (macOS)...");
        scutil_dns().map(|ns| {
            crate::verbose!("[dog]   Found nameserver via scutil: {}", ns);
            ns
        })
    });

    if let Some(ns) = nameserver {
        crate::verbose!("[dog] Resolved nameserver: {}", ns);
        Ok(Resolver { nameserver: ns, search_list })
    } else {
        crate::verbose!("[dog] No system DNS found, falling back to 1.1.1.1");
        Ok(Resolver { nameserver: String::from("1.1.1.1"), search_list })
    }
}

/// Reads the first nameserver from `/etc/resolv.conf`.
#[cfg(unix)]
fn read_resolv_conf() -> io::Result<(String, Vec<String>)> {
    read_resolv_conf_at("/etc/resolv.conf")
}

/// Reads the first nameserver from a resolv.conf-style file at the given path.
#[cfg(unix)]
fn read_resolv_conf_at(path: &str) -> io::Result<(String, Vec<String>)> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::net::IpAddr;

    let f = File::open(path)?;
    let reader = BufReader::new(f);

    let mut search_list = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(ns) = line.strip_prefix("nameserver ") {
            if let Ok(addr) = ns.parse::<IpAddr>() {
                return Ok((addr.to_string(), search_list));
            }
        }
        if let Some(search_str) = line.strip_prefix("search ") {
            search_list.clear();
            search_list.extend(search_str.split_ascii_whitespace().map(|s| s.into()));
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, format!("No nameserver in {}", path)))
}

/// Queries systemd-resolved for DNS servers via `resolvectl dns`.
/// Returns the first physical link DNS if available, falling back to Global DNS.
#[cfg(unix)]
fn resolvectl_dns() -> Option<String> {
    use std::process::Command;
    use std::net::IpAddr;

    let output = Command::new("resolvectl")
        .args(["dns"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let parse_addr = |s: &str| -> Option<String> {
        let s = s.split('#').next().unwrap_or(s);
        let s = s.split('%').next().unwrap_or(s);
        s.parse::<IpAddr>().ok().map(|a| a.to_string())
    };

    let mut global_dns: Option<String> = None;

    for line in stdout.lines() {
        if let Some(colon_pos) = line.find(':') {
            let after = line[colon_pos + 1..].trim();
            if after.is_empty() { continue; }
            if line.starts_with("Global:") {
                global_dns = after.split_whitespace().find_map(&parse_addr);
            } else if line.starts_with("Link ") && !line.contains("(lo)") {
                if let Some(addr) = after.split_whitespace().find_map(&parse_addr) {
                    crate::verbose!("[dog]   resolvectl link DNS: {:?}", addr);
                    return Some(addr);
                }
            }
        }
    }

    crate::verbose!("[dog]   resolvectl falling back to global DNS: {:?}", global_dns);
    global_dns
}

/// Queries macOS's `scutil --dns` for DNS servers.
#[cfg(unix)]
fn scutil_dns() -> Option<String> {
    use std::process::Command;
    use std::net::IpAddr;

    let output = Command::new("scutil")
        .args(["--dns"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(after) = trimmed.strip_prefix("nameserver[") {
            if let Some(colon_pos) = after.find("] : ") {
                let addr_str = after[colon_pos + 4..].trim();
                if let Ok(addr) = addr_str.parse::<IpAddr>() {
                    return Some(addr.to_string());
                }
            }
        }
    }

    None
}


/// Looks up the system default nameserver on Windows, by iterating through
/// the list of network adapters and returning the first nameserver it finds.
#[cfg(windows)]
fn system_nameservers() -> Result<Resolver, ResolverLookupError> {
    use std::net::{IpAddr, UdpSocket};

    if cfg!(test) {
        panic!("system_nameservers() called from test code");
    }

    crate::verbose!("[dog] Looking up system DNS servers...");

    let search_list = Vec::new();

    let adapters = match ipconfig::get_adapters() {
        Ok(a) => {
            crate::verbose!("[dog] Found {} network adapter(s)", a.len());
            for ad in &a {
                crate::verbose!("[dog]   Adapter: {} (up={}, gateways={:?}, ips={:?}, dns={:?})",
                    ad.friendly_name(),
                    ad.oper_status() == ipconfig::OperStatus::IfOperStatusUp,
                    ad.gateways(),
                    ad.ip_addresses(),
                    ad.dns_servers());
            }
            a
        }
        Err(e) => {
            eprintln!("[dog] Error reading adapters: {}", e);
            return Err(ResolverLookupError::Windows(e));
        }
    };

    // First pass: find adapters with DNS servers and gateways
    let active_adapters: Vec<_> = adapters.iter()
        .filter(|a| a.oper_status() == ipconfig::OperStatus::IfOperStatusUp && !a.gateways().is_empty())
        .collect();

    let nameserver = if !active_adapters.is_empty() {
        fn get_primary_ip() -> Option<IpAddr> {
            if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
                if s.connect("8.8.8.8:53").is_ok() {
                    if let Ok(addr) = s.local_addr() {
                        crate::verbose!("[dog] Primary network IP: IPv4 {}", addr.ip());
                        return Some(addr.ip());
                    }
                }
            }
            if let Ok(s) = UdpSocket::bind("[::]:0") {
                if s.connect("[2001:4860:4860::8888]:53").is_ok() {
                    if let Ok(addr) = s.local_addr() {
                        crate::verbose!("[dog] Primary network IP: IPv6 {}", addr.ip());
                        return Some(addr.ip());
                    }
                }
            }
            crate::verbose!("[dog] Could not determine primary network IP");
            None
        }

        if let Some(primary_ip) = get_primary_ip() {
            if let Some(dns) = active_adapters.iter()
                .find(|a| a.ip_addresses().contains(&primary_ip))
                .and_then(|a| a.dns_servers().first())
            {
                crate::verbose!("[dog] Matched primary adapter IP, using DNS: {}", dns);
                Some(dns.to_string())
            } else {
                crate::verbose!("[dog] Primary IP {:?} did not match any adapter", primary_ip);
                None
            }
        } else {
            None
        }
    } else {
        crate::verbose!("[dog] No active adapters with gateways");
        None
    };

    // Fallback sequence
    let nameserver = nameserver.or_else(|| {
        active_adapters.iter()
            .flat_map(|a| a.dns_servers())
            .next()
            .map(|d| {
                crate::verbose!("[dog] Using first DNS from active adapter: {}", d);
                d.to_string()
            })
    }).or_else(|| {
        adapters.iter()
            .filter(|a| a.oper_status() == ipconfig::OperStatus::IfOperStatusUp)
            .flat_map(|a| a.dns_servers())
            .next()
            .map(|d| {
                crate::verbose!("[dog] Using first DNS from any online adapter: {}", d);
                d.to_string()
            })
    });

    if let Some(ns) = nameserver {
        crate::verbose!("[dog] Resolved nameserver: {}", ns);
        return Ok(Resolver { nameserver: ns, search_list });
    }

    crate::verbose!("[dog] No system DNS found, falling back to 1.1.1.1");
    Ok(Resolver { nameserver: String::from("1.1.1.1"), search_list })
}


/// The fall-back system default nameserver determinator that is not very
/// determined as it returns nothing without actually checking anything.
#[cfg(all(not(unix), not(windows)))]
fn system_nameservers() -> Result<Resolver, ResolverLookupError> {
    warn!("Unable to fetch default nameservers on this platform.");
    Err(ResolverLookupError::UnsupportedPlatform)
}


/// Something that can go wrong while obtaining a `Resolver`.
pub enum ResolverLookupError {

    /// The system information was successfully read, but there was no adapter
    /// suitable to use.
    #[allow(dead_code)]
    NoNameserver,

    /// There was an error accessing the network configuration.
    IO(io::Error),

    /// There was an error accessing the network configuration (extra errors
    /// that can only happen on Windows).
    #[cfg(windows)]
    Windows(ipconfig::error::Error),

    /// dog is running on a platform where it doesn't know how to get the
    /// network configuration, so the user must supply one instead.
    #[cfg(all(not(unix), not(windows)))]
    UnsupportedPlatform,
}

impl From<io::Error> for ResolverLookupError {
    fn from(error: io::Error) -> ResolverLookupError {
        Self::IO(error)
    }
}

#[cfg(windows)]
impl From<ipconfig::error::Error> for ResolverLookupError {
    fn from(error: ipconfig::error::Error) -> ResolverLookupError {
        Self::Windows(error)
    }
}

impl fmt::Display for ResolverLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoNameserver => {
                write!(f, "{}", crate::localize::no_nameserver_found())
            }
            Self::IO(ioe) => {
                write!(f, "{}", crate::localize::error_reading_network_config(ioe))
            }
            #[cfg(windows)]
            Self::Windows(ipe) => {
                write!(f, "{}", crate::localize::error_reading_network_config(ipe))
            }
            #[cfg(all(not(unix), not(windows)))]
            Self::UnsupportedPlatform => {
                write!(f, "{}", crate::localize::unsupported_platform())
            }
        }
    }
}
