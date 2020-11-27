#[cfg(not(test))]
pub(crate) use tokio_mock::real::*;

// When testing, we replace all the tokio components with mocks
#[cfg(test)]
pub(crate) use tokio_mock::mock::*;
