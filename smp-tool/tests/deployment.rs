use core::time;
use serde::Deserialize;
use smp_tool::client::Client;
use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

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
async fn test_deployment() -> anyhow::Result<()> {
    let data = fs::read_to_string("../smp-tool/tests/devices.json")?;
    let config: Config = serde_json::from_str(&data)?;

    for dev in config.measurement_devices {
        let addr: SocketAddr = dev.socket_addr.parse()?;
        deploy(addr).await?;
    }

    Ok(())
}

async fn deploy(addr: SocketAddr) -> anyhow::Result<()> {
    println!("Performing DFU on the endpoint: {}", addr);

    let bin_path = PathBuf::from_str("../smp-tool/tests/bin/lcna@3.3.5.bin").unwrap();
    let fw_hash_hex = "1f22547da114895af757c9ddba823a12eb7964bab2946b6534ecaea2f71dca0e";

    common::wait_until_online(addr).await?;
    let hash: String = common::get_hash(addr, 0).await?;
    if fw_hash_hex == hash {
        println!("Already running the target firmware!");
        return Ok(());
    }

    println!("Uploading the image into slot1");

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut client = Client::new(addr, Some(time::Duration::from_millis(5000))).await?;
    // Upload with retry mechanism
    loop {
        let res: std::result::Result<(), String> = client
            .flash(None, &bin_path, 256, false, fw_hash_hex)
            .await
            .map_err(|e| format!("flash error: {e}"));

        match res {
            Ok(()) => {
                println!("Uploading done!");
                break;
            }
            Err(e) => println!("{e}"),
        }

        if Instant::now() >= deadline {
            panic!("Upload failed");
            // or: return Err(anyhow::anyhow!("Upload failed"));
        }
    }

    thread::sleep(Duration::from_secs(1)); // wait after image upload
    println!("Labeling for testing..");

    // label for test + reset via ops
    let res: Result<(), String> = client
        .test_next_boot(&fw_hash_hex)
        .await
        .map_err(|e| format!("test_next_boot error: {e}"));
    println!("Rebooting");

    if let Err(e) = res {
        panic!("image test next boot step failed: {e}");
    }
    let res: Result<(), String> = client
        .reset()
        .await
        .map_err(|e| format!("reset error: {e}"));

    if let Err(e) = res {
        panic!("reset step failed: {e}");
    }

    thread::sleep(Duration::from_secs(1)); // wait after reboot

    common::wait_until_online(addr).await?;

    thread::sleep(Duration::from_secs(1)); // wait before confirming
    println!("Confirming...");

    let res: Result<(), String> = client
        .confirm(&fw_hash_hex)
        .await
        .map_err(|e| format!("confirm error: {e}"));
    if let Err(e) = res {
        panic!("confirm step failed: {e}");
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
