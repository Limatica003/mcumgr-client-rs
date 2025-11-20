// smp-tool/src/ops/os_grp.rs


use crate::error::Result;
use std::net::ToSocketAddrs;

use tracing::debug;

use mcumgr_smp::{
    os_management::{self, EchoResult, ResetResult},
    smp::SmpFrame,
};

use crate::client::Client;

pub fn echo(host: impl ToSocketAddrs, timeout_ms: u64, msg: String) -> Result<()> {
    let mut transport: Client = Client::new(host, timeout_ms)?;
    let ret: SmpFrame<EchoResult> = transport
        .transceive_cbor(&os_management::echo(42, msg))?;
    debug!("{:?}", ret);

    match ret.data {
        EchoResult::Ok { r } => {
            println!("{}", r);
        }
        EchoResult::Err { rc } => {
            eprintln!("rc: {}", rc);
        }
    }
    Ok(())
}

pub fn reset(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<()> {
    let mut transport: Client = Client::new(host, timeout_ms)?;
    let ret: SmpFrame<ResetResult> = transport
        .transceive_cbor(&os_management::reset(42, false))?;
    debug!("{:?}", ret);

    match ret.data {
        ResetResult::Ok {} => {
            println!("Rebooted");
        }
        ResetResult::Err { rc } => {
            eprintln!("rc: {}", rc);
        }
    }
    Ok(())
}
