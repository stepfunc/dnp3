/// Indicates that a task has been shut down
#[derive(Copy, Clone, Debug)]
pub struct Shutdown;

impl std::fmt::Display for Shutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("request could not be completed because the task has shut down")
    }
}

impl std::error::Error for Shutdown {}
