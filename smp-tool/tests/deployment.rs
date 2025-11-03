use anyhow::Ok;
use assert_cmd::{prelude::*};
use std::{env, process::Command};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, net::SocketAddr};
mod common;
use serde::Deserialize;

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
fn test_deployment() -> anyhow::Result<()> {
    let data = fs::read_to_string("../smp-tool/tests/devices.json")?;
    let config: Config = serde_json::from_str(&data)?;

    for dev in config.measurement_devices {
        let addr: SocketAddr = dev.socket_addr.parse()?;
        deploy(&addr.ip().to_string())?;
    }

    Ok(())
}

fn deploy(ip: &str) -> anyhow::Result<()> {
    println!("Performing DFU on the endpoint: {}", ip);
    let mcumgr = assert_cmd::cargo::cargo_bin!("smp-tool");
    //let ip = "192.168.2.101";
    let bin_path = "../smp-tool/tests/bin/lcna@3.3.5.bin";
    let hash = "1f22547da114895af757c9ddba823a12eb7964bab2946b6534ecaea2f71dca0e";
    common::wait_until_online(ip)?;
    println!("Uploading the image into slot1");
    
    let deadline = Instant::now() + Duration::from_secs(20);
    loop { /* Upload with retry mechanism */
        let out =Command::new(mcumgr)
            .args(["-t", "udp", "-d", ip, "app", "flash", bin_path])
            .output()?;
        if out.status.success() { println!("Uploading done!"); break; }             // target is back
        if Instant::now() >= deadline { panic!("Upload failed"); }
    }

    thread::sleep(Duration::from_secs(1)); // wait after image upload
    println!("Labeling for testing..");
    // set pending + reset
    Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "app", "test", "--hash", hash])
        .assert()
        .success();
    
    println!("Rebooting");
    Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "os", "reset"])
        .assert()
        .success();

    thread::sleep(Duration::from_secs(1)); // wait after reboot

    common::wait_until_online(ip)?;

    thread::sleep(Duration::from_secs(1)); // wait before confirming
    println!("Confirming...");
    // confirm
    let out =Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "app", "confirm", "--hash", hash])
        .output()?;
        if out.status.success() {println!("--- app info after deployment ---\n{}", String::from_utf8_lossy(&out.stdout));}
    Ok(())
}