use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::parser::FramePayload;
use crate::outstation::SelfAddressSupport;
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::assembler::{Assembler, AssemblyState};
use crate::transport::header::Header;
use crate::transport::Fragment;

pub(crate) struct Reader {
    link: crate::link::layer::Layer,
    assembler: Assembler,
}

impl Reader {
    fn new(
        endpoint_type: EndpointType,
        self_address: SelfAddressSupport,
        source: EndpointAddress,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(endpoint_type, self_address, source),
            assembler: Assembler::new(),
        }
    }

    pub(crate) fn master(source: EndpointAddress) -> Self {
        Self {
            link: crate::link::layer::Layer::new(
                EndpointType::Master,
                SelfAddressSupport::Disabled,
                source,
            ),
            assembler: Assembler::new(),
        }
    }

    pub(crate) fn outstation(
        source: EndpointAddress,
        self_address_support: SelfAddressSupport,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(
                EndpointType::Outstation,
                self_address_support,
                source,
            ),
            assembler: Assembler::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        self.assembler.peek()
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        // discard any existing frame
        self.assembler.reset();

        let mut payload = FramePayload::new();

        loop {
            let info = self.link.read(io, &mut payload).await?;
            match payload.get() {
                [transport, data @ ..] => {
                    let header = Header::new(*transport);
                    if let AssemblyState::Complete = self.assembler.assemble(info, header, data) {
                        return Ok(());
                    }
                }
                [] => log::warn!("received link data frame with no payload"),
            }
        }
    }
}
