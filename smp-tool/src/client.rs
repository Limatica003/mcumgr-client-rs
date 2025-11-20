// smp-tool/src/client.rs

use std::time::Duration;
use std::net::ToSocketAddrs;

use mcumgr_smp::{
    smp::SmpFrame,
    transport::{
        smp::CborSmpTransport,
        udp::UdpTransport,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use crate::error::Result;

pub struct Client {
    transport: CborSmpTransport,
}

impl Client {
    pub fn new(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<Self> {
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
    ) -> Result<SmpFrame<Resp>>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        Ok(self.transport.transceive_cbor(frame, false)?)
    }
}
