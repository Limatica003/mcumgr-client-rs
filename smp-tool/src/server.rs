// smp-tool/src/server.rs

use std::time::Duration;
use std::net::ToSocketAddrs;

use mcumgr_smp::{
    smp::SmpFrame,
    transport::{
        smp::CborSmpTransport,
        udp::UdpTransport,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use crate::error::Result;

pub struct Server {
    transport: CborSmpTransport,
}

impl Server {
    pub fn new(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<Self> {
        let mut udp = UdpTransport::new_server(host)?;
        udp.recv_timeout(Some(Duration::from_millis(timeout_ms)))?;
        Ok(Self {
            transport: CborSmpTransport {
                transport: Box::new(udp),
            },
        })
    }

    pub fn receive_cbor<Resp>(
        &mut self,
    ) -> Result<SmpFrame<Resp>>
    where
        Resp: DeserializeOwned,
    {
        Ok(self.transport.receive_cbor(None)?)
    }

    pub fn send_to<Req>(&mut self, frame: &SmpFrame<Req>) -> Result<()> 
    where
        Req: Serialize, 
    {
        self.transport.send_to_cbor(frame)?;
        Ok(())
    }
}
