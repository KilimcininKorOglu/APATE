use crate::config::types::RoutingMode;
use crate::routing::table::{Cidr, RouteTable, RouteTarget};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPolicy {
    mode: RoutingMode,
    split_table: RouteTable,
}

impl SplitPolicy {
    pub fn new(mode: RoutingMode) -> Self {
        Self {
            mode,
            split_table: RouteTable::new(RouteTarget::Bypass),
        }
    }

    pub fn add_tunnel_route(&mut self, cidr: Cidr) {
        self.split_table.add_route(cidr, RouteTarget::Tunnel);
    }

    pub fn route_for(&self, destination: IpAddr) -> RouteTarget {
        match self.mode {
            RoutingMode::Full => RouteTarget::Tunnel,
            RoutingMode::Split => self.split_table.lookup(destination),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::types::RoutingMode;
    use crate::routing::{Cidr, RouteTarget, SplitPolicy};
    use std::net::IpAddr;

    #[test]
    fn split_policy_full_mode_always_tunnels() {
        let policy = SplitPolicy::new(RoutingMode::Full);
        let destination = IpAddr::from([203, 0, 113, 1]);

        assert_eq!(RouteTarget::Tunnel, policy.route_for(destination));
    }

    #[test]
    fn split_policy_routes_unlisted_ips_to_bypass() {
        let mut policy = SplitPolicy::new(RoutingMode::Split);
        policy.add_tunnel_route(Cidr::parse("10.0.0.0/8").expect("cidr"));

        let tunnel_ip = IpAddr::from([10, 20, 30, 40]);
        let bypass_ip = IpAddr::from([203, 0, 113, 1]);

        assert_eq!(RouteTarget::Tunnel, policy.route_for(tunnel_ip));
        assert_eq!(RouteTarget::Bypass, policy.route_for(bypass_ip));
    }
}
