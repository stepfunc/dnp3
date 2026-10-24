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
    fn matches(&self, addr: std::net::Ipv4Addr) -> bool {
        fn bm(b: u8, other: Option<u8>) -> bool {
            match other {
                Some(x) => b == x,
                None => true,
            }
        }

        let [b3, b2, b1, b0] = addr.octets();
        bm(b3, self.b3) && bm(b2, self.b2) && bm(b1, self.b1) && bm(b0, self.b0)
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
            AddressFilter::WildcardIpv4(wc) => match addr {
                std::net::IpAddr::V4(x) => wc.matches(x),
                std::net::IpAddr::V6(_) => false,
            },
        }
    }

    /// Whether some address matches both filters
    ///
    /// Every arm names both orders, so swapping the filters selects the same arm, and every named
    /// address is checked with [`AddressFilter::matches`], which canonicalizes it.
    pub(crate) fn conflicts_with(&self, other: &AddressFilter) -> bool {
        match (self, other) {
            (Self::Exact(x), f) | (f, Self::Exact(x)) => f.matches(*x),
            (Self::AnyOf(set), f) | (f, Self::AnyOf(set)) => set.iter().any(|x| f.matches(*x)),
            (Self::Any, _) | (_, Self::Any) => true,
            (Self::WildcardIpv4(a), Self::WildcardIpv4(b)) => a.conflicts_with(b),
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

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn any_of(addrs: &[&str]) -> AddressFilter {
        AddressFilter::AnyOf(addrs.iter().map(|x| ip(x)).collect())
    }

    /// filters written with the IPv4 form of 192.168.0.1 and with its IPv4-mapped IPv6 form; the
    /// two AnyOf filters share no other address, so they overlap only through the two forms
    fn filters_for_192_168_0_1() -> [AddressFilter; 5] {
        [
            AddressFilter::Exact(ip("192.168.0.1")),
            AddressFilter::Exact(ip("::ffff:192.168.0.1")),
            any_of(&["10.0.0.1", "192.168.0.1"]),
            any_of(&["10.0.0.2", "::ffff:192.168.0.1"]),
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

    /// Filter matching and conflicts against a model that decodes the IPv4-mapped prefix from the
    /// octets, independent of `to_canonical`
    ///
    /// - `a_filter_matches_the_canonical_addresses_it_names`: `matches` agrees with the model on
    ///   every address of the universe, and an IPv4 address and its IPv4-mapped form match alike.
    /// - `filters_conflict_when_an_address_matches_both`: `conflicts_with` is true, in both orders,
    ///   exactly when a search of the universe with the model finds an address both filters match.
    ///
    /// Filters of every kind are generated, AnyOf with up to three addresses and wildcards fixing
    /// octets from 0 to 2. Half the conflict pairs are a filter and one naming its addresses in
    /// their other form (native IPv4 and IPv4-mapped swap; a wildcard becomes an IPv4-mapped
    /// address it matches), plus up to two other addresses, so filters that overlap only through
    /// the two forms of an address are common for every kind.
    ///
    /// Known gaps: the universe stands in for all addresses, so octets above 3 and other IPv6
    /// addresses are not drawn, and the IPv4-compatible and IPv4-translated forms (which are not
    /// IPv4-mapped) only appear for 0.1.2.3.
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

        /// 0.1.2.3 in each of its forms and the IPv6 addresses that aren't IPv4-mapped, drawn half
        /// the time so that filters naming a native IPv6 address, or one address in two forms,
        /// are common
        fn forms_of_one_address() -> Vec<IpAddr> {
            let v4 = Ipv4Addr::new(0, 1, 2, 3);
            vec![
                IpAddr::V4(v4),
                IpAddr::V6(v4.to_ipv6_mapped()),
                IpAddr::V6(v4.to_ipv6_compatible()),
                ip("::ffff:0:0.1.2.3"),
                IpAddr::V6(Ipv6Addr::LOCALHOST),
                ip("2001:db8::1"),
            ]
        }

        fn address() -> impl Strategy<Value = IpAddr> {
            prop_oneof![
                prop::sample::select(universe()),
                prop::sample::select(forms_of_one_address()),
            ]
        }

        fn wildcard_octet() -> impl Strategy<Value = Option<u8>> {
            prop_oneof![Just(None), (0u8..3).prop_map(Some)]
        }

        fn filter() -> impl Strategy<Value = AddressFilter> {
            prop_oneof![
                1 => Just(AddressFilter::Any),
                1 => address().prop_map(AddressFilter::Exact),
                1 => prop::collection::vec(address(), 0..4)
                    .prop_map(|x| AddressFilter::AnyOf(x.into_iter().collect())),
                // twice as often, so that pairs of wildcards overlapping only through each
                // other's wildcard octets are common
                2 => (
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

        /// the same address in its other form: native IPv4 and IPv4-mapped swap, others stay
        fn other_form(addr: IpAddr) -> IpAddr {
            match addr {
                IpAddr::V4(x) => IpAddr::V6(x.to_ipv6_mapped()),
                IpAddr::V6(x) => x.to_ipv4_mapped().map_or(addr, IpAddr::V4),
            }
        }

        /// A filter naming the addresses `filter` names in their other form, plus `extra`
        ///
        /// A wildcard names the IPv4-mapped form of the address it matches with its wildcard
        /// octets taken from `fill`. Any names no particular address, so it gets an AnyOf of
        /// `extra` alone, which is empty a third of the time.
        fn restated(filter: &AddressFilter, extra: Vec<IpAddr>, fill: [u8; 4]) -> AddressFilter {
            let named = match filter {
                AddressFilter::Any => return AddressFilter::AnyOf(extra.into_iter().collect()),
                AddressFilter::AnyOf(set) => {
                    let named = set.iter().map(|x| other_form(*x));
                    return AddressFilter::AnyOf(named.chain(extra).collect());
                }
                AddressFilter::Exact(x) => other_form(*x),
                AddressFilter::WildcardIpv4(wc) => {
                    let [b3, b2, b1, b0] = fill;
                    let v4 = Ipv4Addr::new(
                        wc.b3.unwrap_or(b3),
                        wc.b2.unwrap_or(b2),
                        wc.b1.unwrap_or(b1),
                        wc.b0.unwrap_or(b0),
                    );
                    IpAddr::V6(v4.to_ipv6_mapped())
                }
            };
            if extra.is_empty() {
                AddressFilter::Exact(named)
            } else {
                AddressFilter::AnyOf(std::iter::once(named).chain(extra).collect())
            }
        }

        /// two independent filters half the time, otherwise a filter and [`restated`] of it
        fn pair() -> impl Strategy<Value = (AddressFilter, AddressFilter)> {
            let restated_pair = (
                filter(),
                prop::collection::vec(address(), 0..3),
                prop::array::uniform4(prop::sample::select(OCTETS.to_vec())),
            )
                .prop_map(|(lhs, extra, fill)| {
                    let rhs = restated(&lhs, extra, fill);
                    (lhs, rhs)
                });
            prop_oneof![(filter(), filter()), restated_pair]
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(256))]

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
            fn filters_conflict_when_an_address_matches_both((lhs, rhs) in pair()) {
                let witness = universe()
                    .into_iter()
                    .find(|addr| model_matches(&lhs, *addr) && model_matches(&rhs, *addr));
                prop_assert_eq!(lhs.conflicts_with(&rhs), witness.is_some(), "{:?}", witness);
                prop_assert_eq!(rhs.conflicts_with(&lhs), witness.is_some(), "{:?}", witness);
            }
        }
    }
}
