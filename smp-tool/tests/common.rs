use mcumgr_smp::application_management::{self, GetImageStateResult};
use mcumgr_smp::smp::SmpFrame;
use smp_tool::client::Client;
use std::time::{Duration, Instant};
use anyhow::anyhow;

pub fn wait_until_online(ip: &str) -> anyhow::Result<()> {
    println!("Trying to connect...");
    let deadline = Instant::now() + Duration::from_secs(20);

    loop {
        let ok = // per-attempt timeout ~1 s
            match Client::new((ip.to_string(), 1337), 1000) {
                Ok(mut client) => {
                    let res: Result<SmpFrame<GetImageStateResult>, _> =
                        client
                            .transceive_cbor(&application_management::get_state(42));
                    res.is_ok()
                }
                Err(_) => false,
            };

        if ok {
            println!("Connected!");
            break;
        }

        if Instant::now() >= deadline {
            panic!("target is not available!");
        }
    }

    Ok(())
}

pub fn get_hash(ip: String, slot: i32) -> anyhow::Result<String> {
    println!("Fetching the hash of the image on slot{slot}");

    let addr = (ip.clone(), 1337);

    // fetch hash of given slot
    let mut client = Client::new(addr, 5000).map_err(|e| anyhow!(e.to_string()))?;
    let frame: SmpFrame<GetImageStateResult> =
        client
            .transceive_cbor(&application_management::get_state(42))
            .map_err(|e| anyhow!(e.to_string()))?;

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