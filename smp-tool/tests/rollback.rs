use serde::Deserialize;
use smp_tool::client::Client;

use std::{
    fs,
    net::SocketAddr,
    thread,
    time::Duration,
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

    // get hash for slot 1
    let hash = common::get_hash(ip.to_string(), 1)?;
    let addr = (ip.to_string(), 1337);
    let mut client = Client::new(addr, 5000)?;
    println!("Labeling for testing..");
    
    // set pending + reset via ops (all sync now)
    let res: Result<(), String> = (|| -> Result<(), String> {
        client.test_next_boot(&hash)
            .map_err(|e| format!("test_next_boot error: {e}"))?;

        println!("Rebooting");
        client.reset()
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
        client.confirm(&hash)
            .map_err(|e| format!("confirm error: {e}"))?;
        Ok(())
    })();
    if let Err(e) = res {
        panic!("confirm step failed: {e}");
    }

    let res: Result<(), String> = 
        client.info()
            .map_err(|e| format!("app info error: {e}"));
    
    if let Err(e) = res {
        panic!("app final info step failed: {e}");
    }

    Ok(())
}