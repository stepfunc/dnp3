//use crate::outstation::task::OutstationTask;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AcceptAction {
    DropNew,
    DropExisting,
}

pub trait AddressFilter {
    fn filter(address: &std::net::SocketAddr) -> Option<AcceptAction>;
}

/*
pub enum TaskState {
    Idle(OutstationTask),
    Running(crate::tokio::task::JoinHandle<OutstationTask>),
}

struct TaskRecord {

}

struct TaskMap {
    map: std::HashMap<u16, >
}

fn x() {
    crate::tokio::s
}
*/
