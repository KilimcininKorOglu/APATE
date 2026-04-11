use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteTarget {
    Tunnel,
    Bypass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix_len: u8,
}

impl Cidr {
    pub fn parse(value: &str) -> Result<Self, RouteTableError> {
        let (network_str, prefix_str) =
            value
                .split_once('/')
                .ok_or_else(|| RouteTableError::InvalidCidr {
                    value: String::from(value),
                })?;

        let network = network_str
            .parse::<IpAddr>()
            .map_err(|_| RouteTableError::InvalidCidr {
                value: String::from(value),
            })?;
        let prefix_len = prefix_str
            .parse::<u8>()
            .map_err(|_| RouteTableError::InvalidCidr {
                value: String::from(value),
            })?;

        let max_prefix = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix_len > max_prefix {
            return Err(RouteTableError::InvalidPrefix {
                value: prefix_len,
                max: max_prefix,
            });
        }

        let normalized_network = normalize_network(network, prefix_len);
        Ok(Self {
            network: normalized_network,
            prefix_len,
        })
    }

    pub fn contains(&self, address: IpAddr) -> bool {
        match (self.network, address) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                if self.prefix_len == 0 {
                    return true;
                }
                let network_u32 = u32::from(network);
                let address_u32 = u32::from(address);
                let mask = u32::MAX << (32 - self.prefix_len);
                (network_u32 & mask) == (address_u32 & mask)
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                if self.prefix_len == 0 {
                    return true;
                }
                let network_u128 = u128::from(network);
                let address_u128 = u128::from(address);
                let mask = u128::MAX << (128 - self.prefix_len);
                (network_u128 & mask) == (address_u128 & mask)
            }
            _ => false,
        }
    }

    pub fn prefix_len(&self) -> u8 {
        self.prefix_len
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub cidr: Cidr,
    pub target: RouteTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTable {
    default_target: RouteTarget,
    entries: Vec<RouteEntry>,
}

impl RouteTable {
    pub fn new(default_target: RouteTarget) -> Self {
        Self {
            default_target,
            entries: Vec::new(),
        }
    }

    pub fn add_route(&mut self, cidr: Cidr, target: RouteTarget) {
        self.entries.push(RouteEntry { cidr, target });
        self.entries
            .sort_by(|left, right| right.cidr.prefix_len().cmp(&left.cidr.prefix_len()));
    }

    pub fn add_route_from_str(
        &mut self,
        cidr: &str,
        target: RouteTarget,
    ) -> Result<(), RouteTableError> {
        self.add_route(Cidr::parse(cidr)?, target);
        Ok(())
    }

    pub fn lookup(&self, address: IpAddr) -> RouteTarget {
        self.entries
            .iter()
            .find(|entry| entry.cidr.contains(address))
            .map_or(self.default_target, |entry| entry.target)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RouteTableError {
    #[error("invalid cidr: {value}")]
    InvalidCidr { value: String },
    #[error("invalid cidr prefix: {value}, max is {max}")]
    InvalidPrefix { value: u8, max: u8 },
}

fn normalize_network(network: IpAddr, prefix_len: u8) -> IpAddr {
    match network {
        IpAddr::V4(ip) => {
            if prefix_len == 0 {
                return IpAddr::from([0, 0, 0, 0]);
            }
            let mask = u32::MAX << (32 - prefix_len);
            IpAddr::V4((u32::from(ip) & mask).into())
        }
        IpAddr::V6(ip) => {
            if prefix_len == 0 {
                return IpAddr::from([0u8; 16]);
            }
            let mask = u128::MAX << (128 - prefix_len);
            IpAddr::V6((u128::from(ip) & mask).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::routing::table::{Cidr, RouteTable, RouteTarget};
    use std::net::IpAddr;

    #[test]
    fn route_table_uses_longest_prefix_match() {
        let mut table = RouteTable::new(RouteTarget::Bypass);
        table
            .add_route_from_str("10.0.0.0/8", RouteTarget::Bypass)
            .expect("add /8");
        table
            .add_route_from_str("10.10.0.0/16", RouteTarget::Tunnel)
            .expect("add /16");

        let matched = table.lookup(IpAddr::from([10, 10, 1, 1]));
        let unmatched = table.lookup(IpAddr::from([11, 10, 1, 1]));

        assert_eq!(RouteTarget::Tunnel, matched);
        assert_eq!(RouteTarget::Bypass, unmatched);
    }

    #[test]
    fn cidr_parse_rejects_invalid_prefix() {
        let parsed = Cidr::parse("10.0.0.0/33");
        assert!(parsed.is_err());
    }

    #[test]
    fn cidr_contains_ipv6_addresses() {
        let cidr = Cidr::parse("2001:db8::/32").expect("cidr");
        let match_ip = "2001:db8::1".parse::<IpAddr>().expect("ip");
        let miss_ip = "2001:db9::1".parse::<IpAddr>().expect("ip");

        assert!(cidr.contains(match_ip));
        assert!(!cidr.contains(miss_ip));
    }
}
