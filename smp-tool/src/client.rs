// smp-tool/src/client.rs

use core::time;
use std::path::Path;
use tokio::net::{ToSocketAddrs};

use mcumgr_smp::{
    smp::SmpFrame,
    transport::{
        smp::CborSmpTransportAsync,
        udp::UdpTransportAsync,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use crate::ops::{os_grp, shell_grp};
use crate::{error::Result, ops::img_grp};

pub struct Client {
    transport: CborSmpTransportAsync,
}

impl Client {
    pub async fn new(host: impl ToSocketAddrs, timeout: Option<time::Duration>) -> Result<Self> {
        let udp = UdpTransportAsync::new(&host, timeout).await?;
        Ok(Self {
            transport: CborSmpTransportAsync {
                transport: Box::new(udp),
            },
        })
    }

    pub async fn transceive_cbor<Req, Resp>(
        &mut self,
        frame: &SmpFrame<Req>,
    ) -> Result<SmpFrame<Resp>>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        Ok(self.transport.transceive_cbor(frame, false).await?)
    }

    // --------------- IMG GRP --------------- 

    pub async fn info(&mut self) -> Result<()> {
        img_grp::info(self).await
    }

    pub async fn flash(
        &mut self,
        slot: Option<u8>,
        update_file: &Path,
        chunk_size: usize,
        upgrade: bool,
        hash: &str
    ) -> Result<()> {
        img_grp::flash(self, slot, update_file, chunk_size, upgrade, hash).await
    }

    pub async fn confirm(&mut self, hash_hex: &str) -> Result<()> {
        img_grp::confirm(self, hash_hex).await
    }

    pub async fn test_next_boot(&mut self, hash_hex: &str) -> Result<()> {
        img_grp::test_next_boot(self, hash_hex).await
    }

    // --------------- OS GRP ---------------

    pub async fn echo(&mut self, msg: String) -> Result<()> {
        os_grp::echo(self, msg).await
    }
    
    pub async fn reset(&mut self) -> Result<()> {
        os_grp::reset(self).await
    }

    // --------------- SHELL GRP ---------------

    pub async fn transceive(&mut self, cmd: Vec<String>) ->  Result<String> {
        shell_grp::transceive(self, cmd).await
    }

    pub async fn exec(&mut self, cmd: Vec<String>) -> Result<()> {
        shell_grp::exec(self, cmd).await
    }

    pub async fn interactive(&mut self) -> Result<()> {
        shell_grp::interactive(self).await
    }

}
