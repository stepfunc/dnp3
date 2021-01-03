pub mod tcp;

#[derive(Clone, Debug, PartialEq)]
pub enum AddressFilter {
    Any,
    Exact(std::net::SocketAddr),
    AnyOf(std::collections::HashSet<std::net::SocketAddr>),
}

impl AddressFilter {
    pub(crate) fn matches(&self, addr: std::net::SocketAddr) -> bool {
        match self {
            AddressFilter::Any => true,
            AddressFilter::Exact(x) => *x == addr,
            AddressFilter::AnyOf(set) => set.contains(&addr),
        }
    }

    pub(crate) fn conflicts_with(&self, other: &AddressFilter) -> bool {
        match self {
            AddressFilter::Any => true,
            AddressFilter::Exact(x) => other.matches(*x),
            AddressFilter::AnyOf(set) => set.iter().any(|x| other.matches(*x)),
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
