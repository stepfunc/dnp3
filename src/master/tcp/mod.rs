use crate::master::session::SessionMap;

use crate::app::parse::parser::ParseLogLevel;
use crate::master::runner::{MasterHandle, RunError, Runner};
use crate::transport::{ReaderType, WriterType};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;

pub struct MasterTask {
    endpoint: SocketAddr,
    runner: Runner,
    reader: ReaderType,
    writer: WriterType,
}

impl MasterTask {
    pub fn new(
        address: u16,
        level: ParseLogLevel,
        response_timeout: Duration,
        endpoint: SocketAddr,
        sessions: SessionMap,
    ) -> (Self, MasterHandle) {
        let (runner, handle) = Runner::new(level, response_timeout, sessions);
        let (reader, writer) = crate::transport::create_transport_layer(true, address);
        let task = Self {
            endpoint,
            runner,
            reader,
            writer,
        };
        (task, handle)
    }

    pub async fn run(&mut self) {
        loop {
            match TcpStream::connect(self.endpoint).await {
                Err(err) => {
                    log::warn!("{}", err);
                }
                Ok(mut socket) => {
                    match self
                        .runner
                        .run(&mut socket, &mut self.writer, &mut self.reader)
                        .await
                    {
                        RunError::Shutdown => return,
                        RunError::Link(err) => {
                            log::warn!("{}", err);
                        }
                    }
                }
            }

            self.reader.reset();
            self.writer.reset();

            // TODO - implement a reconnect delay

            // TODO - do we need to reset the runner?
            //runner.reset();
        }
    }
}

/*
async fn spawn_master_task(address: u16, level: ParseLogLevel, timeout: Duration, endpoint: SocketAddr, sessions: SessionMap) -> MasterHandle  {
    let (mut task, handle) = MasterTask::new(address, level, timeout, endpoint, sessions);
    tokio::spawn(async move { task.run() });
    handle
}
*/
