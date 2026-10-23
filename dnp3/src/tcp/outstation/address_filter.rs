// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::str::FromStr;

/// Represents IPv4 addresses which may contain "*" wildcards
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WildcardIPv4 {
    pub(crate) b3: Option<u8>,
    pub(crate) b2: Option<u8>,
    pub(crate) b1: Option<u8>,
    pub(crate) b0: Option<u8>,
}

/// Error returned when an IPv4 wildcard is not in the correct format
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BadIpv4Wildcard;

fn get_byte(value: &str) -> Result<Option<u8>, BadIpv4Wildcard> {
    match value {
        "*" => Ok(None),
        _ => match value.parse::<u8>() {
            Ok(x) => Ok(Some(x)),
            Err(_) => Err(BadIpv4Wildcard),
        },
    }
}

impl FromStr for WildcardIPv4 {
    type Err = BadIpv4Wildcard;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split('.');
        let b3 = get_byte(iter.next().ok_or(BadIpv4Wildcard)?)?;
        let b2 = get_byte(iter.next().ok_or(BadIpv4Wildcard)?)?;
        let b1 = get_byte(iter.next().ok_or(BadIpv4Wildcard)?)?;
        let b0 = get_byte(iter.next().ok_or(BadIpv4Wildcard)?)?;

        if iter.next().is_some() {
            return Err(BadIpv4Wildcard);
        }

        Ok(WildcardIPv4 { b3, b2, b1, b0 })
    }
}

impl WildcardIPv4 {
    pub(crate) fn matches(&self, addr: std::net::IpAddr) -> bool {
        fn bm(b: u8, other: Option<u8>) -> bool {
            match other {
                Some(x) => b == x,
                None => true,
            }
        }

        match addr.to_canonical() {
            std::net::IpAddr::V4(x) => {
                let [b3, b2, b1, b0] = x.octets();
                bm(b3, self.b3) && bm(b2, self.b2) && bm(b1, self.b1) && bm(b0, self.b0)
            }
            std::net::IpAddr::V6(_) => false,
        }
    }

    pub(crate) fn conflicts_with(&self, other: &WildcardIPv4) -> bool {
        fn matches(lhs: Option<u8>, rhs: Option<u8>) -> bool {
            match (lhs, rhs) {
                (Some(x), Some(y)) => x == y,
                // if either of the values is a wildcard, then it conflicts
                _ => true,
            }
        }
        // wildcards conflict if all the bytes match
        matches(self.b3, other.b3)
            && matches(self.b2, other.b2)
            && matches(self.b1, other.b1)
            && matches(self.b0, other.b0)
    }
}

/// Address filter used to control which master address(es) may connect to an outstation.
///
/// An IPv4 address and its IPv4-mapped IPv6 form (`::ffff:a.b.c.d`) are the same address, so IPv4
/// filters match IPv4 masters on a server bound to an IPv6 address such as `[::]`.
///
/// Two filters conflict when at least one address matches both. An empty [`AddressFilter::AnyOf`]
/// matches no address, so it conflicts with no filter, including [`AddressFilter::Any`].
///
/// Note: User code not exhaustively match against this enum as new variants may be added in the future.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddressFilter {
    /// allow any address
    Any,
    /// allow a specific address
    Exact(std::net::IpAddr),
    /// allow any of set of addresses (an empty set allows none)
    AnyOf(std::collections::HashSet<std::net::IpAddr>),
    /// matches against an IPv4 address with wildcards
    WildcardIpv4(WildcardIPv4),
}

impl AddressFilter {
    pub(crate) fn matches(&self, addr: std::net::IpAddr) -> bool {
        let addr = addr.to_canonical();
        match self {
            AddressFilter::Any => true,
            AddressFilter::Exact(x) => x.to_canonical() == addr,
            AddressFilter::AnyOf(set) => match addr {
                std::net::IpAddr::V4(v4) => {
                    set.contains(&addr) || set.contains(&std::net::IpAddr::V6(v4.to_ipv6_mapped()))
                }
                std::net::IpAddr::V6(_) => set.contains(&addr),
            },
            AddressFilter::WildcardIpv4(wc) => wc.matches(addr),
        }
    }

    pub(crate) fn conflicts_with(&self, other: &AddressFilter) -> bool {
        match self {
            AddressFilter::Any => match other {
                AddressFilter::AnyOf(set) => !set.is_empty(),
                AddressFilter::Any | AddressFilter::Exact(_) | AddressFilter::WildcardIpv4(_) => {
                    true
                }
            },
            AddressFilter::Exact(x) => other.matches(*x),
            AddressFilter::AnyOf(set) => set.iter().any(|x| other.matches(*x)),
            AddressFilter::WildcardIpv4(wc) => match other {
                AddressFilter::Any => true,
                AddressFilter::Exact(x) => wc.matches(*x),
                AddressFilter::AnyOf(set) => set.iter().any(|x| wc.matches(*x)),
                AddressFilter::WildcardIpv4(wc2) => wc.conflicts_with(wc2),
            },
        }
    }
}

/// error type returned when a filter conflicts with another filter
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FilterError {
    /// filter conflicts with an existing filter
    Conflict,
}

impl std::error::Error for FilterError {}

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FilterError::Conflict => f.write_str("filter conflicts with an existing filter"),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tcp::{AddressFilter, BadIpv4Wildcard, WildcardIPv4};
    use std::net::IpAddr;

    #[test]
    fn parses_address_with_subnet_wildcard() {
        let wc: WildcardIPv4 = "172.17.20.*".parse().unwrap();
        assert_eq!(
            wc,
            WildcardIPv4 {
                b3: Some(172),
                b2: Some(17),
                b1: Some(20),
                b0: None
            }
        )
    }

    #[test]
    fn parses_all_wildcards() {
        let wc: WildcardIPv4 = "*.*.*.*".parse().unwrap();
        assert_eq!(
            wc,
            WildcardIPv4 {
                b3: None,
                b2: None,
                b1: None,
                b0: None
            }
        )
    }

    #[test]
    fn rejects_bad_input() {
        let bad_input = [
            "*.*.*.*.*",
            "*.*..*.*",
            "*.256.*.*",
            ".*.256.*.*",
            "1.1.1.1ab",
        ];

        for x in bad_input {
            let res: Result<WildcardIPv4, BadIpv4Wildcard> = x.parse();
            assert_eq!(res, Err(BadIpv4Wildcard));
        }

        let res: Result<WildcardIPv4, BadIpv4Wildcard> = "*.*.*.*.*".parse();
        assert_eq!(res, Err(BadIpv4Wildcard));
    }

    #[test]
    fn wildcard_matching_works() {
        let wc: WildcardIPv4 = "192.168.0.*".parse().unwrap();
        let ip1: IpAddr = "192.168.0.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(wc.matches(ip1));
        assert!(!wc.matches(ip2));
    }

    #[test]
    fn wildcard_conflicts() {
        let wc1 = AddressFilter::WildcardIpv4("192.168.27.*".parse().unwrap());
        let ip1 = AddressFilter::Exact("192.168.27.2".parse().unwrap());

        let wc2 = AddressFilter::WildcardIpv4("192.168.*.23".parse().unwrap());
        let ip2 = AddressFilter::Exact("192.168.22.23".parse().unwrap());

        // conflicts
        assert!(wc1.conflicts_with(&ip1));
        assert!(wc1.conflicts_with(&wc2));

        // disjoint
        assert!(!wc1.conflicts_with(&ip2));
        assert!(!wc2.conflicts_with(&ip1));
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn any_of(addrs: &[&str]) -> AddressFilter {
        AddressFilter::AnyOf(addrs.iter().map(|x| ip(x)).collect())
    }

    /// filters written with the IPv4 form of 192.168.0.1 and with its IPv4-mapped IPv6 form
    fn filters_for_192_168_0_1() -> [AddressFilter; 5] {
        [
            AddressFilter::Exact(ip("192.168.0.1")),
            AddressFilter::Exact(ip("::ffff:192.168.0.1")),
            any_of(&["10.0.0.1", "192.168.0.1"]),
            any_of(&["10.0.0.1", "::ffff:192.168.0.1"]),
            AddressFilter::WildcardIpv4("192.168.0.*".parse().unwrap()),
        ]
    }

    #[test]
    fn native_and_mapped_ipv4_peers_match_either_form() {
        for filter in filters_for_192_168_0_1() {
            for peer in ["192.168.0.1", "::ffff:192.168.0.1"] {
                assert!(filter.matches(ip(peer)), "{filter:?} {peer}");
            }
            for peer in ["192.168.1.1", "::ffff:192.168.1.1", "::192.168.0.1", "::1"] {
                assert!(!filter.matches(ip(peer)), "{filter:?} {peer}");
            }
        }
    }

    #[test]
    fn native_ipv6_matching_is_unchanged() {
        let filter = any_of(&["2001:db8::1", "192.168.0.1"]);
        assert!(filter.matches(ip("2001:db8::1")));
        assert!(!filter.matches(ip("2001:db8::2")));
        assert!(AddressFilter::Exact(ip("2001:db8::1")).matches(ip("2001:db8::1")));
        assert!(!AddressFilter::Exact(ip("2001:db8::1")).matches(ip("2001:db8::2")));
    }

    #[test]
    fn native_and_mapped_forms_of_an_address_conflict() {
        for lhs in filters_for_192_168_0_1() {
            for rhs in filters_for_192_168_0_1() {
                assert!(lhs.conflicts_with(&rhs), "{lhs:?} {rhs:?}");
            }
        }
    }

    #[test]
    fn an_empty_any_of_conflicts_with_no_filter_in_either_order() {
        let empty = AddressFilter::AnyOf(Default::default());
        let others = [
            AddressFilter::Any,
            AddressFilter::Exact(ip("192.168.0.1")),
            any_of(&["192.168.0.1"]),
            AddressFilter::AnyOf(Default::default()),
            AddressFilter::WildcardIpv4("*.*.*.*".parse().unwrap()),
        ];
        for other in others.iter() {
            assert!(!empty.conflicts_with(other), "{empty:?} {other:?}");
            assert!(!other.conflicts_with(&empty), "{other:?} {empty:?}");
        }
    }

    #[test]
    fn mapped_forms_of_different_addresses_do_not_conflict() {
        let others = [
            AddressFilter::Exact(ip("::ffff:192.168.1.1")),
            any_of(&["::ffff:192.168.1.1", "2001:db8::1"]),
            AddressFilter::Exact(ip("::192.168.0.1")),
        ];
        for lhs in filters_for_192_168_0_1() {
            for rhs in others.iter() {
                assert!(!lhs.conflicts_with(rhs), "{lhs:?} {rhs:?}");
                assert!(!rhs.conflicts_with(&lhs), "{rhs:?} {lhs:?}");
            }
        }
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;
        use std::net::{Ipv4Addr, Ipv6Addr};

        const OCTETS: [u8; 4] = [0, 1, 2, 3];

        /// Every IPv4 address with octets from 0 to 3 in its native and IPv4-mapped forms, and
        /// IPv6 addresses that aren't IPv4-mapped. Wildcards only fix octets from 0 to 2, so the
        /// universe holds an address matched by both filters whenever two filters overlap.
        fn universe() -> Vec<IpAddr> {
            let mut addresses = vec![
                IpAddr::V6(Ipv6Addr::LOCALHOST),
                ip("2001:db8::1"),
                // IPv4-compatible and IPv4-translated forms are not IPv4-mapped
                IpAddr::V6(Ipv4Addr::new(0, 1, 2, 3).to_ipv6_compatible()),
                ip("::ffff:0:0.1.2.3"),
            ];
            for a in OCTETS {
                for b in OCTETS {
                    for c in OCTETS {
                        for d in OCTETS {
                            let v4 = Ipv4Addr::new(a, b, c, d);
                            addresses.push(IpAddr::V4(v4));
                            addresses.push(IpAddr::V6(v4.to_ipv6_mapped()));
                        }
                    }
                }
            }
            addresses
        }

        /// the IPv4 octets an address denotes, decoding the IPv4-mapped prefix from the bytes
        fn ipv4_octets(addr: IpAddr) -> Option<[u8; 4]> {
            match addr {
                IpAddr::V4(x) => Some(x.octets()),
                IpAddr::V6(x) => match x.octets() {
                    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, a, b, c, d] => Some([a, b, c, d]),
                    _ => None,
                },
            }
        }

        fn same_address(lhs: IpAddr, rhs: IpAddr) -> bool {
            match (ipv4_octets(lhs), ipv4_octets(rhs)) {
                (Some(x), Some(y)) => x == y,
                (None, None) => lhs == rhs,
                _ => false,
            }
        }

        fn model_matches(filter: &AddressFilter, addr: IpAddr) -> bool {
            match filter {
                AddressFilter::Any => true,
                AddressFilter::Exact(x) => same_address(*x, addr),
                AddressFilter::AnyOf(set) => set.iter().any(|x| same_address(*x, addr)),
                AddressFilter::WildcardIpv4(wc) => ipv4_octets(addr).is_some_and(|octets| {
                    [wc.b3, wc.b2, wc.b1, wc.b0]
                        .into_iter()
                        .zip(octets)
                        .all(|(fixed, octet)| fixed.is_none_or(|fixed| fixed == octet))
                }),
            }
        }

        fn address() -> impl Strategy<Value = IpAddr> {
            prop::sample::select(universe())
        }

        fn wildcard_octet() -> impl Strategy<Value = Option<u8>> {
            prop_oneof![Just(None), (0u8..3).prop_map(Some)]
        }

        fn filter() -> impl Strategy<Value = AddressFilter> {
            prop_oneof![
                Just(AddressFilter::Any),
                address().prop_map(AddressFilter::Exact),
                prop::collection::vec(address(), 0..4)
                    .prop_map(|x| AddressFilter::AnyOf(x.into_iter().collect())),
                (
                    wildcard_octet(),
                    wildcard_octet(),
                    wildcard_octet(),
                    wildcard_octet()
                )
                    .prop_map(|(b3, b2, b1, b0)| AddressFilter::WildcardIpv4(
                        WildcardIPv4 { b3, b2, b1, b0 }
                    )),
            ]
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]

            #[test]
            fn a_filter_matches_the_canonical_addresses_it_names(filter in filter()) {
                for addr in universe() {
                    prop_assert_eq!(
                        filter.matches(addr),
                        model_matches(&filter, addr),
                        "{:?}",
                        addr
                    );
                }
                // an IPv4 address and its IPv4-mapped form are the same address
                for addr in universe() {
                    if let IpAddr::V4(v4) = addr {
                        let mapped = IpAddr::V6(v4.to_ipv6_mapped());
                        prop_assert_eq!(filter.matches(addr), filter.matches(mapped));
                    }
                }
            }

            #[test]
            fn filters_conflict_when_an_address_matches_both(lhs in filter(), rhs in filter()) {
                let witness = universe()
                    .into_iter()
                    .find(|addr| model_matches(&lhs, *addr) && model_matches(&rhs, *addr));
                prop_assert_eq!(lhs.conflicts_with(&rhs), witness.is_some(), "{:?}", witness);
                prop_assert_eq!(rhs.conflicts_with(&lhs), witness.is_some(), "{:?}", witness);
            }
        }
    }
}
