use crate::tcp::Endpoint;

/// List of IP endpoints
///
/// You can write IP addresses or DNS names and the port to connect to. e.g. `"127.0.0.1:20000"` or `"dnp3.myorg.com:20000"`.
///
/// The main endpoint is always favoured. When a successful connection is established, it resets the next endpoint
/// to try, meaning if the connection is lost, the main endpoint will be retried first, then the other endpoints
/// in the order they were defined.
#[derive(Clone, Debug)]
pub struct EndpointList {
    endpoints: Vec<String>,
}

impl EndpointList {
    pub(crate) fn endpoints(self) -> Vec<Endpoint> {
        self.endpoints.into_iter().map(Endpoint::from).collect()
    }

    /// Create a list with a single endpoint
    pub fn single(addr: String) -> Self {
        Self::new(addr, &[])
    }

    /// Create a list with a main endpoint followed by fail-overs
    pub fn new(addr: String, fail_overs_list: &[String]) -> Self {
        let mut endpoints = vec![addr];
        endpoints.extend_from_slice(fail_overs_list);
        Self { endpoints }
    }

    /// Add an IP endpoint
    pub fn add(&mut self, addr: String) {
        self.endpoints.push(addr);
    }
}
