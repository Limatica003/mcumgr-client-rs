// smp-tool/src/client.rs

use std::time::Duration;
use std::net::ToSocketAddrs;

use mcumgr_smp::{
    smp::SmpFrame,
    transport::{
        error::Error as TransportError,
        smp::CborSmpTransport,
        udp::UdpTransport,
    },
};
use serde::{de::DeserializeOwned, Serialize};

pub struct Client {
    transport: CborSmpTransport,
}

impl Client {
    pub fn set_transport(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<Self, TransportError> {
        let mut udp = UdpTransport::new(host)?;
        udp.recv_timeout(Some(Duration::from_millis(timeout_ms)))?;
        Ok(Self {
            transport: CborSmpTransport {
                transport: Box::new(udp),
            },
        })
    }

    pub fn transceive_cbor<Req, Resp>(
        &mut self,
        frame: &SmpFrame<Req>,
    ) -> Result<SmpFrame<Resp>, TransportError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        self.transport.transceive_cbor(frame, false)
    }
}
