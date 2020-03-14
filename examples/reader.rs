use dnp3rs::transport::reader::Reader;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let mut listener = TcpListener::bind(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut socket, _) = listener.accept().await?;

    let mut reader = Reader::new(false, 1024);

    loop {

        let asdu = reader.read(&mut socket).await.unwrap();

        log::info!(
            "received asdu w/ length {} from {:?}",
            asdu.data.len(),
            asdu.address.source
        )

    }
}
