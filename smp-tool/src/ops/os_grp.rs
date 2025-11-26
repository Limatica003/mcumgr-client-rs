// smp-tool/src/ops/os_grp.rs


use crate::error::Result;

use tracing::debug;

use mcumgr_smp::{
    os_management::{self, EchoResult, ResetResult},
    smp::SmpFrame,
};

use crate::client::Client;

pub async fn echo(transport: &mut Client, msg: String) -> Result<()> {
    let ret: SmpFrame<EchoResult> = transport
        .transceive_cbor(&os_management::echo(42, msg)).await?;
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

pub async fn reset(transport: &mut Client) -> Result<()> {
    let ret: SmpFrame<ResetResult> = transport
        .transceive_cbor(&os_management::reset(42, false)).await?;
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
