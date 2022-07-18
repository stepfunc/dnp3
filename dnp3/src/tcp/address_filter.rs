/// Represents IPv4 addresses which may contain "*" wildcards
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WildcardIPv4 {
    pub(crate) b3: Option<u8>,
    pub(crate) b2: Option<u8>,
    pub(crate) b1: Option<u8>,
    pub(crate) b0: Option<u8>,
}

/// Error returned when an IPv4 wildcard is not in the correct format
#[derive(Debug, Copy, Clone)]
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

impl TryFrom<&str> for WildcardIPv4 {
    type Error = BadIpv4Wildcard;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut iter = value.split('.');
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

        match addr {
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
        // wildcards conflict if all of the bytes match
        matches(self.b3, other.b3)
            && matches(self.b2, other.b2)
            && matches(self.b1, other.b1)
            && matches(self.b0, other.b0)
    }
}

/// Address filter used to control which master address(es) may connect to an outstation.
///
/// Note: User code not exhaustively match against this enum as new variants may be added in the future.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum AddressFilter {
    /// allow any address
    Any,
    /// allow a specific address
    Exact(std::net::IpAddr),
    /// allow any of set of addresses
    AnyOf(std::collections::HashSet<std::net::IpAddr>),
    /// matches against an IPv4 address with wildcards
    WildcardIpv4(WildcardIPv4),
}

impl AddressFilter {
    pub(crate) fn matches(&self, addr: std::net::IpAddr) -> bool {
        match self {
            AddressFilter::Any => true,
            AddressFilter::Exact(x) => *x == addr,
            AddressFilter::AnyOf(set) => set.contains(&addr),
            AddressFilter::WildcardIpv4(wc) => wc.matches(addr),
        }
    }

    pub(crate) fn conflicts_with(&self, other: &AddressFilter) -> bool {
        match self {
            AddressFilter::Any => true,
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
#[derive(Copy, Clone, Debug, PartialEq)]
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
