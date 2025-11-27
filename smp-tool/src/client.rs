// smp-tool/src/client.rs

use core::time;
use std::path::Path;
use tokio::net::ToSocketAddrs;
use std::sync::atomic::{AtomicU8, Ordering};

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
    seq: AtomicU8,
}

impl Client {
    pub async fn new(host: impl ToSocketAddrs, timeout: Option<time::Duration>) -> Result<Self> {
        let udp = UdpTransportAsync::new(&host, timeout).await?;
        Ok(Self {
            transport: CborSmpTransportAsync {
                transport: Box::new(udp),
            },
            seq: 0.into(),
        })
    }

    fn next_seq(&self) -> u8 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn transceive_cbor<Req, Resp>(
        &mut self,
        frame: &SmpFrame<Req>,
    ) -> Result<SmpFrame<Resp>>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        Ok(self.transport.transceive_cbor(frame, true).await?)
    }

    // --------------- IMG GRP --------------- 

    pub async fn info(&mut self) -> Result<()> {
        let seq = self.next_seq();
        img_grp::info(self, seq).await
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
        let seq = self.next_seq();
        img_grp::confirm(self, hash_hex, seq).await
    }

    pub async fn test_next_boot(&mut self, hash_hex: &str) -> Result<()> {
        let seq = self.next_seq();
        img_grp::test_next_boot(self, hash_hex, seq).await
    }

    // --------------- OS GRP ---------------

    pub async fn echo(&mut self, msg: String) -> Result<()> {
        let seq = self.next_seq();
        os_grp::echo(self, msg, seq).await
    }
    
    pub async fn reset(&mut self) -> Result<()> {
        let seq = self.next_seq();
        os_grp::reset(self, seq).await
    }

    // --------------- SHELL GRP ---------------

    pub async fn transceive(&mut self, cmd: Vec<String>) ->  Result<String> {
        let seq = self.next_seq();
        shell_grp::transceive(self, cmd, seq).await
    }

    pub async fn exec(&mut self, cmd: Vec<String>) -> Result<()> {
        let seq = self.next_seq();
        shell_grp::exec(self, cmd, seq).await
    }

    pub async fn interactive(&mut self) -> Result<()> {
        let seq = self.next_seq();
        shell_grp::interactive(self, seq).await
    }

}
