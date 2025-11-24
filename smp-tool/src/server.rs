// smp-tool/src/server.rs

use std::time::Duration;
use std::net::ToSocketAddrs;

use mcumgr_smp::{
    shell_management::{self, ShellCommand}, smp::SmpFrame, transport::{
        smp::CborSmpTransport,
        udp::UdpTransport,
    }
};
use serde::{Serialize};
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

    pub fn send_to_cbor<Req>(&mut self, frame: &SmpFrame<Req>) -> Result<()> 
    where
        Req: Serialize, 
    {
        self.transport.send_to_cbor(frame)?;
        Ok(())
    }

    /// This function listens the smp client
    pub fn receive(&mut self) ->  Result<String> {
        let ret: SmpFrame<ShellCommand> = self.transport.receive_cbor(None)?;

        let argv = ret.data.argv;
        Ok(argv.join(" "))
    }

    
    /// Reply to the client which responds lately
    pub fn reply(&mut self, cmd: String) ->  Result<()> 
    {
        self.transport.send_to_cbor(&shell_management::shell_command(42, vec![cmd]))?;
        Ok(())
    }

}
