use dnp3rs::app::gen::enums::FunctionCode;
use dnp3rs::app::parse::parser::{parse_request, HeaderParser, ParseType};
use dnp3rs::transport::reader::Reader;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;

fn parse_asdu(data: &[u8]) {
    let request = match parse_request(data) {
        Err(e) => {
            log::warn!("bad request: {:?}", e);
            return;
        }
        Ok(request) => request,
    };

    let mode: ParseType = match request.function {
        FunctionCode::Read => ParseType::Read,
        _ => ParseType::NonRead,
    };

    if let Err(e) = HeaderParser::two_pass(mode, request.objects) {
        log::warn!("bad header: {:?}", e);
    }
}

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let mut listener = TcpListener::bind(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut socket, _) = listener.accept().await?;

    let mut reader = Reader::new(false, 1024);

    loop {
        let asdu = reader.read(&mut socket).await.unwrap();

        parse_asdu(asdu.data)
    }
}
