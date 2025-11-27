use core::time;
use mcumgr_smp::application_management::{self, GetImageStateResult};
use mcumgr_smp::smp::SmpFrame;
use smp_tool::client::Client;
use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};

pub async fn wait_until_online(host: SocketAddr) -> Result<()> {
    println!("Trying to connect...");
    let deadline = Instant::now() + Duration::from_secs(20);

    let mut client = Client::new(host, Some(time::Duration::from_millis(1000))).await?;

    loop {
        if Instant::now() >= deadline {
            return Err(anyhow!("target is not available!"));
        }

        let res: std::result::Result<SmpFrame<GetImageStateResult>, _> = client
            .transceive_cbor(&application_management::get_state(42))
            .await;

        if res.is_ok() {
            println!("Connected!");
            return Ok(());
        }
    }
}

pub async fn get_hash(addr: SocketAddr, slot: i32) -> anyhow::Result<String> {
    println!("Fetching the hash of the image on slot{slot}");

    // fetch hash of given slot
    let mut client = Client::new(addr, Some(time::Duration::from_millis(3000))).await?;
    let frame: SmpFrame<GetImageStateResult> = client
        .transceive_cbor(&application_management::get_state(42))
        .await?;

    let hash = match frame.data {
        GetImageStateResult::Ok(payload) => {
            let mut slot_hash: Option<String> = None;

            for img in payload.images {
                if img.slot == slot {
                    if let Some(h) = img.hash {
                        if h.len() == 32 {
                            let s: String = h.iter().map(|b| format!("{:02x}", b)).collect();
                            slot_hash = Some(s);
                            break;
                        }
                    }
                }
            }

            slot_hash.ok_or_else(|| anyhow!("slot {slot} hash not found"))?
        }
        GetImageStateResult::Err(err) => {
            return Err(anyhow!(
                "GetImageStateResult error rc={}, rsn={:?}",
                err.rc,
                err.rsn
            ));
        }
    };
    Ok(hash)
}
