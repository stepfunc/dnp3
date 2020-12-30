use crate::entry::outstation::{AddressFilter, Match};

pub fn any_address(priority: u32) -> Box<dyn AddressFilter> {
    Box::new(AnyAddress { priority })
}

pub fn exact_match_v4(address: std::net::Ipv4Addr, priority: u32) -> Box<dyn AddressFilter> {
    Box::new(ExactMatchV4 { address, priority })
}

struct AnyAddress {
    priority: u32,
}

struct ExactMatchV4 {
    address: std::net::Ipv4Addr,
    priority: u32,
}

impl AddressFilter for AnyAddress {
    fn matches(&self, _: &std::net::SocketAddr) -> Match {
        Match::yes(self.priority)
    }
}

impl AddressFilter for ExactMatchV4 {
    fn matches(&self, address: &std::net::SocketAddr) -> Match {
        match *address {
            std::net::SocketAddr::V4(x) => {
                if self.address == *x.ip() {
                    Match::yes(self.priority)
                } else {
                    Match::no()
                }
            }
            _ => Match::no(),
        }
    }
}
