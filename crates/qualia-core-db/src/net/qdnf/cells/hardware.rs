//! Process RSS and NUMA observation (E10.5).
//!
//! Sentinel pass accounting is not RSS. Idle joules stay unmeasured unless a
//! RAPL energy counter is sampled as a delta, which this library does not do.

use super::scaling::HardwareObservation;

/// Linux `/proc/self/status` VmRSS, converted to bytes.
pub fn rss_bytes() -> Result<u64, HardwareObservation> {
    #[cfg(target_os = "linux")]
    {
        parse_vmrss_bytes(&read_limited("/proc/self/status")?)
            .ok_or(HardwareObservation::Unmeasured)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(HardwareObservation::Unsupported)
    }
}

/// First NUMA node that holds pages for this process, else the first present node.
pub fn numa_node() -> Result<u32, HardwareObservation> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(maps) = read_limited("/proc/self/numa_maps") {
            if let Some(n) = parse_numa_maps_node(&maps) {
                return Ok(n);
            }
        }
        first_sysfs_node().ok_or(HardwareObservation::Unmeasured)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(HardwareObservation::Unsupported)
    }
}

/// Idle energy is a duration delta. A RAPL counter snapshot is not idle joules.
pub fn idle_joules() -> Result<u64, HardwareObservation> {
    let _ = rapl_energy_uj();
    Err(HardwareObservation::Unmeasured)
}

/// RAPL energy counter if present. Not an idle-energy measurement.
pub fn rapl_energy_uj() -> Result<u64, HardwareObservation> {
    #[cfg(target_os = "linux")]
    {
        parse_u64_file("/sys/class/powercap/intel-rapl:0/energy_uj")
            .or_else(|| parse_u64_file("/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj"))
            .ok_or(HardwareObservation::Unsupported)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(HardwareObservation::Unsupported)
    }
}

#[inline]
pub fn rss_measured() -> bool {
    rss_bytes().is_ok()
}

#[inline]
pub fn numa_measured() -> bool {
    numa_node().is_ok()
}

#[inline]
pub const fn idle_energy_measured() -> bool {
    false
}

const PROC_CAP: usize = 8192;

fn read_limited(path: &str) -> Result<String, HardwareObservation> {
    let bytes = std::fs::read(path).map_err(|_| HardwareObservation::Unmeasured)?;
    if bytes.len() > PROC_CAP {
        return Err(HardwareObservation::Unmeasured);
    }
    String::from_utf8(bytes).map_err(|_| HardwareObservation::Unmeasured)
}

fn parse_vmrss_bytes(status: &str) -> Option<u64> {
    for line in status.lines() {
        let Some(rest) = line.strip_prefix("VmRSS:") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let n = parts.next()?.parse::<u64>().ok()?;
        return Some(n.saturating_mul(1024));
    }
    None
}

fn parse_numa_maps_node(maps: &str) -> Option<u32> {
    let mut best_node = 0u32;
    let mut best_pages = 0u64;
    let mut found = false;
    for line in maps.lines() {
        let mut rest = line;
        while let Some(i) = rest.find('N') {
            rest = &rest[i + 1..];
            let digits_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if digits_end == 0 {
                continue;
            }
            if !rest[digits_end..].starts_with('=') {
                continue;
            }
            let Ok(node) = rest[..digits_end].parse::<u32>() else {
                rest = &rest[digits_end..];
                continue;
            };
            let after_eq = &rest[digits_end + 1..];
            let val_end = after_eq
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after_eq.len());
            let pages: u64 = after_eq[..val_end].parse().unwrap_or(0);
            if !found || pages > best_pages {
                best_node = node;
                best_pages = pages;
                found = true;
            }
            rest = &rest[digits_end..];
        }
    }
    if found {
        Some(best_node)
    } else {
        None
    }
}

fn first_sysfs_node() -> Option<u32> {
    let mut n = 0u32;
    while n < 8 {
        let mut path = [0u8; 40];
        let s = format_node_path(&mut path, n)?;
        if std::path::Path::new(s).is_dir() {
            return Some(n);
        }
        n = n + 1;
    }
    None
}

fn format_node_path<'a>(buf: &'a mut [u8], n: u32) -> Option<&'a str> {
    let prefix = b"/sys/devices/system/node/node";
    if buf.len() < prefix.len() + 2 {
        return None;
    }
    buf[..prefix.len()].copy_from_slice(prefix);
    if n > 9 {
        return None;
    }
    buf[prefix.len()] = b'0' + n as u8;
    core::str::from_utf8(&buf[..prefix.len() + 1]).ok()
}

fn parse_u64_file(path: &str) -> Option<u64> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::cells::pass_budget::pass_maps_process_rss;

    #[test]
    fn vmrss_parser_reads_kib_as_bytes() {
        let sample = "Name:\tinit\nVmRSS:\t   42 kB\n";
        assert_eq!(parse_vmrss_bytes(sample), Some(42 * 1024));
        assert_eq!(parse_vmrss_bytes("Name:\tinit\n"), None);
    }

    #[test]
    fn numa_maps_parser_picks_largest_node() {
        let maps = "00400000 default file=/bin/a N0=3 N1=9\n";
        assert_eq!(parse_numa_maps_node(maps), Some(1));
        assert_eq!(parse_numa_maps_node("00400000 default\n"), None);
    }

    #[test]
    fn linux_rss_and_numa_are_sampled_or_unmeasured() {
        assert!(!pass_maps_process_rss());
        assert!(!idle_energy_measured());
        assert_eq!(idle_joules(), Err(HardwareObservation::Unmeasured));
        #[cfg(target_os = "linux")]
        {
            match rss_bytes() {
                Ok(n) => {
                    assert!(n > 0);
                    assert!(rss_measured());
                }
                Err(HardwareObservation::Unmeasured) => assert!(!rss_measured()),
                Err(other) => panic!("linux RSS must not be {:?}", other),
            }
            match numa_node() {
                Ok(_) => assert!(numa_measured()),
                Err(HardwareObservation::Unmeasured) => assert!(!numa_measured()),
                Err(other) => panic!("linux NUMA must not be {:?}", other),
            }
            match rapl_energy_uj() {
                Ok(_) | Err(HardwareObservation::Unsupported) => {}
                Err(other) => panic!("RAPL snapshot must not be {:?}", other),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(rss_bytes(), Err(HardwareObservation::Unsupported));
            assert_eq!(numa_node(), Err(HardwareObservation::Unsupported));
            assert!(!rss_measured());
            assert!(!numa_measured());
        }
    }
}
