use std::collections::VecDeque;
use std::net::SocketAddr;

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
    pending_endpoints: VecDeque<SocketAddr>,
    current_endpoint: usize,
}

impl EndpointList {
    /// Create a list with a single endpoint
    pub fn single(addr: String) -> Self {
        Self::new(addr, &[])
    }

    /// Create a list with a main endpoint followed by fail-overs
    pub fn new(addr: String, fail_overs_list: &[String]) -> Self {
        let mut endpoints = vec![addr];
        endpoints.extend_from_slice(fail_overs_list);
        Self {
            endpoints,
            pending_endpoints: VecDeque::new(),
            current_endpoint: 0,
        }
    }

    /// Add an IP endpoint
    pub fn add(&mut self, addr: String) {
        self.endpoints.push(addr);
    }

    /// Returns the first (main) address of the list
    pub fn main_addr(&self) -> &str {
        self.endpoints.first().unwrap()
    }

    pub(crate) fn reset(&mut self) {
        self.pending_endpoints.clear();
        self.current_endpoint = 0;
    }

    pub(crate) async fn next_address(&mut self) -> Option<SocketAddr> {
        if let Some(endpoint) = self.pending_endpoints.pop_front() {
            return Some(endpoint);
        }

        let start_idx = self.current_endpoint;

        loop {
            let endpoint_idx = self.current_endpoint;

            // Increment the current endpoint
            self.current_endpoint = (self.current_endpoint + 1) % self.endpoints.len();

            // Resolve the name
            if let Ok(endpoints) = tokio::net::lookup_host(&self.endpoints[endpoint_idx]).await {
                self.pending_endpoints = endpoints.collect();

                if let Some(endpoint) = self.pending_endpoints.pop_front() {
                    return Some(endpoint);
                }
            } else {
                tracing::warn!("unable to resolve \"{}\"", &self.endpoints[endpoint_idx]);
            }

            // We tried every possibility, but haven't found anything
            if start_idx == self.current_endpoint {
                return None;
            }
        }
    }
}
