// smp-tool/src/server.rs

use std::net::SocketAddr;

use tokio::net::ToSocketAddrs;

use crate::error::Result;
use mcumgr_smp::{
    application_management,
    shell_management::{self, ShellCommand},
    smp::SmpFrame,
    transport::{smp::CborSmpTransportAsync, udp::UdpTransportAsync},
    Group,
};
use serde::Serialize;

pub struct Server {
    transport: CborSmpTransportAsync,
    target_grp: Group,
    pub local_addr: SocketAddr,
}

impl Server {
    pub async fn new(host: impl ToSocketAddrs) -> Result<Self> {
        let udp = UdpTransportAsync::new_server(host).await?;
        let local_addr = udp.local_addr;
        Ok(Self {
            transport: CborSmpTransportAsync {
                transport: Box::new(udp),
            },
            target_grp: Group::Default,
            local_addr,
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
    pub async fn receive(&mut self) -> Result<String> {
        let res = self.transport.receive_cbor::<ShellCommand>(None).await;
        match res {
            Ok(frame) => {
                // Shell case
                self.target_grp = frame.group; // should be Group::ShellManagement
                let argv = frame.data.argv;
                Ok(argv.join(" "))
            }
            Err(_) => {
                self.target_grp = Group::ApplicationManagement;
                Ok("app_management_msg_received".to_string())
            }
        }
    }

    /// Reply to the client which responds lately
    pub async fn reply(&mut self, cmd: String) -> Result<()> {
        if self.target_grp == Group::ApplicationManagement {
            self.transport
                .send_to_cbor(&application_management::get_state_response(42, cmd))
                .await?;
        } else {
            self.transport
                .send_to_cbor(&shell_management::shell_command_response(42, cmd))
                .await?;
        }

        Ok(())
    }
}
