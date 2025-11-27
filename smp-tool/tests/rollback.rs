use serde::Deserialize;
use smp_tool::client::Client;

use core::time;
use std::{fs, net::SocketAddr, thread, time::Duration};

mod common;

#[derive(Deserialize)]
struct Config {
    measurement_devices: Vec<Device>,
}

#[derive(Deserialize)]
struct Device {
    socket_addr: String,
}

#[tokio::test]
#[ignore] // run manually
async fn test_rollback() -> anyhow::Result<()> {
    let data = fs::read_to_string("../smp-tool/tests/devices.json")?;
    let config: Config = serde_json::from_str(&data)?;

    for dev in config.measurement_devices {
        let addr: SocketAddr = dev.socket_addr.parse()?;
        rollback(addr).await?;
    }

    Ok(())
}

async fn rollback(addr: SocketAddr) -> anyhow::Result<()> {
    println!("Performing rollback on the endpoint: {}", addr);

    common::wait_until_online(addr).await?;

    // get hash for slot 1
    let hash = common::get_hash(addr, 1).await?;
    let mut client = Client::new(addr, Some(time::Duration::from_millis(5000))).await?;
    println!("Labeling for testing..");

    // set pending + reset via ops (all sync now)
    let res: Result<(), String> = async {
        client
            .test_next_boot(&hash)
            .await
            .map_err(|e| format!("test_next_boot error: {e}"))?;

        println!("Rebooting");
        client
            .reset()
            .await
            .map_err(|e| format!("reset error: {e}"))?;

        Ok(())
    }
    .await;
    if let Err(e) = res {
        panic!("label/reset step failed: {e}");
    }

    thread::sleep(Duration::from_secs(1)); // wait after reboot
    common::wait_until_online(addr).await?;
    thread::sleep(Duration::from_secs(1)); // wait before confirming

    println!("Confirming...");

    let res: Result<(), String> = async {
        client
            .confirm(&hash)
            .await
            .map_err(|e| format!("confirm error: {e}"))?;
        Ok(())
    }
    .await;

    if let Err(e) = res {
        panic!("app confirm step failed: {e}");
    }

    let res: Result<(), String> = client
        .info()
        .await
        .map_err(|e| format!("app info error: {e}"));

    if let Err(e) = res {
        panic!("app final info step failed: {e}");
    }

    Ok(())
}
