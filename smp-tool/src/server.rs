// smp-tool/src/server.rs

use std::net::SocketAddr;

use serde_json::Value as CborValue;
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
    seq: u8,
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
            seq: 0,
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
        // 1) Decode header + payload generically so we can read frame.group
        let frame_any: SmpFrame<CborValue> = self.transport.receive_cbor::<CborValue>(None).await?;

        self.target_grp = frame_any.group;
        self.seq = frame_any.sequence;

        // 2) Dispatch by group
        match self.target_grp {
            Group::ShellManagement => {
                // Option A (simple, a bit wasteful): re-decode into the typed payload
                let bytes = frame_any.encode_with_cbor();
                let frame = SmpFrame::<ShellCommand>::decode_with_cbor(&bytes)?;
                Ok(frame.data.argv.join(" "))
            }

            Group::ApplicationManagement => {
                // If you have an AppMgmt request type, decode it here the same way.
                // Otherwise keep it minimal as you asked:
                Ok("app_management_msg_received".to_string())
            }

            _ => Ok(String::new()),
        }
    }

    /// Reply to the client which responds lately
    pub async fn reply(&mut self, cmd: String) -> Result<()> {
        if self.target_grp == Group::ApplicationManagement {
            self.transport
                .send_to_cbor(&application_management::get_state_response(self.seq, cmd))
                .await?;
        } else {
            self.transport
                .send_to_cbor(&shell_management::shell_command_response(self.seq, cmd))
                .await?;
        }

        Ok(())
    }
}
