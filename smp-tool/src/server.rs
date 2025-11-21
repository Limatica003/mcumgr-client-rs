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
use serde::{de::DeserializeOwned};
use crate::error::Result;

pub struct Server {
    transport: CborSmpTransport,
}

impl Server {
    pub fn new(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<Self> {
        let mut udp = UdpTransport::new(host)?;
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
}
