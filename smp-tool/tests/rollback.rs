use serde::Deserialize;
use smp_tool::client::Client;
use smp_tool::ops::{img_grp, os_grp};
use anyhow::anyhow;

use std::{
    fs,
    net::SocketAddr,
    thread,
    time::Duration,
};

use mcumgr_smp::application_management::{self, GetImageStateResult};
use mcumgr_smp::smp::SmpFrame;

mod common;

#[derive(Deserialize)]
struct Config {
    measurement_devices: Vec<Device>,
}

#[derive(Deserialize)]
struct Device {
    socket_addr: String,
}

#[test]
#[ignore] // run manually
fn test_rollback() -> anyhow::Result<()> {
    let data = fs::read_to_string("../smp-tool/tests/devices.json")?;
    let config: Config = serde_json::from_str(&data)?;

    for dev in config.measurement_devices {
        let addr: SocketAddr = dev.socket_addr.parse()?;
        rollback(&addr.ip().to_string())?;
    }

    Ok(())
}

fn rollback(ip: &str) -> anyhow::Result<()> {
    println!("Performing rollback on the endpoint: {}", ip);

    common::wait_until_online(ip)?;
    println!("Fetching the hash of the image on slot1");

    // we use the same addr everywhere
    let addr = (ip.to_string(), 1337);

    // fetch hash of slot 1 via SMP (sync)
    let mut client = Client::set_transport(addr.clone(), 5000).map_err(|e| anyhow!(e.to_string()))?;
    let frame: SmpFrame<GetImageStateResult> =
        client.transceive_cbor(&application_management::get_state(42)).map_err(|e| anyhow!(e.to_string()))?;

    let hash: String = match frame.data {
        GetImageStateResult::Ok(payload) => {
            let mut slot1_hash: Option<String> = None;

            for img in payload.images {
                if img.slot == 1 {
                    if let Some(h) = img.hash {
                        if h.len() == 32 {
                            let s: String = h.iter().map(|b| format!("{:02x}", b)).collect();
                            slot1_hash = Some(s);
                            break;
                        }
                    }
                }
            }

            slot1_hash.ok_or_else(|| anyhow::anyhow!("slot 1 hash not found"))?
        }
        GetImageStateResult::Err(err) => {
            return Err(anyhow::anyhow!(
                "GetImageStateResult error rc={}, rsn={:?}",
                err.rc,
                err.rsn
            ));
        }
    };

    println!("Labeling for testing..");

    // set pending + reset via ops (all sync now)
    let res: Result<(), String> = (|| -> Result<(), String> {
        img_grp::test_next_boot(&addr, 5000, &hash)
            .map_err(|e| format!("test_next_boot error: {e}"))?;

        println!("Rebooting");
        os_grp::reset(addr.clone(), 5000)
            .map_err(|e| format!("reset error: {e}"))?;

        Ok(())
    })();
    if let Err(e) = res {
        panic!("label/reset step failed: {e}");
    }

    thread::sleep(Duration::from_secs(1)); // wait after reboot
    common::wait_until_online(ip)?;
    thread::sleep(Duration::from_secs(1)); // wait before confirming

    println!("Confirming...");

    let res: Result<(), String> = (|| -> Result<(), String> {
        img_grp::confirm(addr, 5000, &hash)
            .map_err(|e| format!("confirm error: {e}"))?;
        Ok(())
    })();
    if let Err(e) = res {
        panic!("confirm step failed: {e}");
    }

    let res: Result<(), String> = 
        img_grp::info((ip.to_string(), 1337), 1000)
            .map_err(|e| format!("app info error: {e}"));
    
    if let Err(e) = res {
        panic!("app final info step failed: {e}");
    }
    
    Ok(())
}