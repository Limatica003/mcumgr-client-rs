// smp-tool/src/client.rs

use std::{path::Path, time::Duration};
use std::net::{SocketAddr};

use mcumgr_smp::{
    smp::SmpFrame,
    transport::{
        smp::CborSmpTransport,
        udp::UdpTransport,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use crate::ops::{os_grp, shell_grp};
use crate::{error::Result, ops::img_grp};

pub struct Client {
    transport: CborSmpTransport,
}

impl Client {
    pub fn new(host: SocketAddr, timeout_ms: u64) -> Result<Self> {
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

    // --------------- IMG GRP --------------- 

    pub fn info(&mut self) -> Result<()> {
        img_grp::info(self)
    }

    pub fn flash(
        &mut self,
        slot: Option<u8>,
        update_file: &Path,
        chunk_size: usize,
        upgrade: bool,
        hash: &str
    ) -> Result<()> {
        img_grp::flash(self, slot, update_file, chunk_size, upgrade, hash)
    }

    pub fn confirm(&mut self, hash_hex: &str) -> Result<()> {
        img_grp::confirm(self, hash_hex)
    }

    pub fn test_next_boot(&mut self, hash_hex: &str) -> Result<()> {
        img_grp::test_next_boot(self, hash_hex)
    }

    // --------------- OS GRP ---------------

    pub fn echo(&mut self, msg: String) -> Result<()> {
        os_grp::echo(self, msg)
    }
    
    pub fn reset(&mut self) -> Result<()> {
        os_grp::reset(self)
    }

    // --------------- SHELL GRP ---------------

    pub fn transceive(&mut self, cmd: Vec<String>) ->  Result<String> {
        shell_grp::transceive(self, cmd)
    }

    pub fn exec(&mut self, cmd: Vec<String>) -> Result<()> {
        shell_grp::exec(self, cmd)
    }

    pub fn interactive(&mut self) -> Result<()> {
        shell_grp::interactive(self)
    }

}
