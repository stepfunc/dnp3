use dnp3rs::app::header::ResponseFunction;
use dnp3rs::app::parse::parser::Response;
use dnp3rs::app::sequence::Sequence;
use dnp3rs::transport::reader::Reader;
use dnp3rs::transport::writer::Writer;
use dnp3rs::util::cursor::WriteCursor;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpStream;

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let mut socket = TcpStream::connect(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let mut reader = Reader::new(true, 1);
    let mut writer = Writer::new(1, true);

    let mut buffer: [u8; 100] = [0; 100];
    let mut seq = Sequence::default();

    loop {
        let mut cursor = WriteCursor::new(&mut buffer);
        dnp3rs::app::format::write::read_integrity(seq, &mut cursor).unwrap();
        writer
            .write(&mut socket, 1024, cursor.written())
            .await
            .unwrap();
        seq.increment();

        loop {
            let response = reader.read(&mut socket).await.unwrap();

            match Response::parse(response.data) {
                Err(err) => {
                    log::warn!("bad response: {:?}", err);
                    break;
                }
                Ok(response) => {
                    if response.header.function == ResponseFunction::Solicited {
                        if response.header.control.con {
                            let mut cursor = WriteCursor::new(&mut buffer);
                            dnp3rs::app::format::write::confirm_solicited(
                                response.header.control.seq,
                                &mut cursor,
                            )
                            .unwrap();
                            writer
                                .write(&mut socket, 1024, cursor.written())
                                .await
                                .unwrap();
                        }

                        if response.header.control.fin {
                            break;
                        }
                    }

                    if let Err(err) = response.parse_objects() {
                        log::warn!("bad response object: {:?}", err);
                        break;
                    }
                }
            }
        }
    }
}
