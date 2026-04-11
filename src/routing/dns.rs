use crate::config::types::DnsMode;
use crate::routing::table::RouteTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsAction {
    UseDoh,
    UsePlain,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsPolicy {
    mode: DnsMode,
    doh_available: bool,
}

impl DnsPolicy {
    pub fn new(mode: DnsMode, doh_available: bool) -> Self {
        Self {
            mode,
            doh_available,
        }
    }

    pub fn route_dns_query(&self, route_target: RouteTarget) -> DnsAction {
        match self.mode {
            DnsMode::Plain => DnsAction::UsePlain,
            DnsMode::Doh => {
                if self.doh_available {
                    DnsAction::UseDoh
                } else {
                    DnsAction::Blocked
                }
            }
            DnsMode::Fallback => match route_target {
                RouteTarget::Tunnel => {
                    if self.doh_available {
                        DnsAction::UseDoh
                    } else {
                        DnsAction::UsePlain
                    }
                }
                RouteTarget::Bypass => {
                    if self.doh_available {
                        DnsAction::UseDoh
                    } else {
                        DnsAction::Blocked
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::types::DnsMode;
    use crate::routing::dns::{DnsAction, DnsPolicy};
    use crate::routing::table::RouteTarget;

    #[test]
    fn dns_policy_blocks_doh_mode_without_endpoint() {
        let policy = DnsPolicy::new(DnsMode::Doh, false);
        assert_eq!(
            DnsAction::Blocked,
            policy.route_dns_query(RouteTarget::Tunnel)
        );
    }

    #[test]
    fn dns_policy_prevents_plain_fallback_on_bypass_when_protected() {
        let policy = DnsPolicy::new(DnsMode::Fallback, false);
        assert_eq!(
            DnsAction::Blocked,
            policy.route_dns_query(RouteTarget::Bypass)
        );
    }

    #[test]
    fn dns_policy_uses_plain_mode_when_explicitly_configured() {
        let policy = DnsPolicy::new(DnsMode::Plain, false);
        assert_eq!(
            DnsAction::UsePlain,
            policy.route_dns_query(RouteTarget::Bypass)
        );
    }
}
