// smp-tool/src/server.rs

use tokio::net::ToSocketAddrs;

use mcumgr_smp::{
    shell_management::{self, ShellCommand}, smp::SmpFrame, transport::{
        smp::CborSmpTransportAsync,
        udp::UdpTransportAsync,
    }
};
use serde::{Serialize};
use crate::error::Result;

pub struct Server {
    transport: CborSmpTransportAsync,
}

impl Server {
    pub async fn new(host: impl ToSocketAddrs) -> Result<Self> {
        let udp = UdpTransportAsync::new_server(host).await?;
        Ok(Self {
            transport: CborSmpTransportAsync {
                transport: Box::new(udp),
            },
        })
    }

    pub async fn send_to_cbor<Req>(&mut self, frame: &SmpFrame<Req>) -> Result<()> 
    where
        Req: Serialize, 
    {
        self.transport.send_to_cbor(frame).await?;
        Ok(())
    }

    /// This function listens the smp client
    pub async fn receive(&mut self) ->  Result<String> {
        let ret: SmpFrame<ShellCommand> = self.transport.receive_cbor(None).await?;

        let argv = ret.data.argv;
        Ok(argv.join(" "))
    }

    
    /// Reply to the client which responds lately
    pub async fn reply(&mut self, cmd: String) ->  Result<()> 
    {
        self.transport.send_to_cbor(&shell_management::shell_command_response(42, vec![cmd])).await?;
        Ok(())
    }

}
