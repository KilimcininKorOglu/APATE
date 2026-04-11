pub mod dns;
pub mod split;
pub mod table;

pub use dns::{DnsAction, DnsPolicy};
pub use split::SplitPolicy;
pub use table::{Cidr, RouteTable, RouteTableError, RouteTarget};

use crate::config::types::{DnsMode, RoutingMode};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingEngine {
    split_policy: SplitPolicy,
    dns_policy: DnsPolicy,
}

impl RoutingEngine {
    pub fn new(mode: RoutingMode, dns_mode: DnsMode, doh_available: bool) -> Self {
        Self {
            split_policy: SplitPolicy::new(mode),
            dns_policy: DnsPolicy::new(dns_mode, doh_available),
        }
    }

    pub fn add_split_tunnel_route(&mut self, cidr: Cidr) {
        self.split_policy.add_tunnel_route(cidr);
    }

    pub fn route_packet(&self, destination: IpAddr) -> RouteTarget {
        self.split_policy.route_for(destination)
    }

    pub fn route_dns_query(&self, destination: IpAddr) -> DnsAction {
        let route_target = self.route_packet(destination);
        self.dns_policy.route_dns_query(route_target)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::types::{DnsMode, RoutingMode};
    use crate::routing::{Cidr, DnsAction, RouteTarget, RoutingEngine};
    use std::net::IpAddr;

    #[test]
    fn full_mode_sends_all_packets_to_tunnel() {
        let engine = RoutingEngine::new(RoutingMode::Full, DnsMode::Doh, true);
        let ip = IpAddr::from([8, 8, 8, 8]);

        assert_eq!(RouteTarget::Tunnel, engine.route_packet(ip));
        assert_eq!(DnsAction::UseDoh, engine.route_dns_query(ip));
    }

    #[test]
    fn split_mode_uses_route_table_for_decisions() {
        let mut engine = RoutingEngine::new(RoutingMode::Split, DnsMode::Fallback, true);
        engine.add_split_tunnel_route(Cidr::parse("10.0.0.0/8").expect("cidr"));

        let tunnel_ip = IpAddr::from([10, 1, 2, 3]);
        let bypass_ip = IpAddr::from([1, 1, 1, 1]);

        assert_eq!(RouteTarget::Tunnel, engine.route_packet(tunnel_ip));
        assert_eq!(RouteTarget::Bypass, engine.route_packet(bypass_ip));
        assert_eq!(DnsAction::UseDoh, engine.route_dns_query(tunnel_ip));
        assert_eq!(DnsAction::UseDoh, engine.route_dns_query(bypass_ip));
    }
}
