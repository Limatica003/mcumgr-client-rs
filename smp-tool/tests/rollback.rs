use assert_cmd::{prelude::*};
use std::{env, process::Command};
use std::thread;
use std::time::{Duration};
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
    let mcumgr = assert_cmd::cargo::cargo_bin!("smp-tool");
    //let ip = "192.168.2.101";

    common::wait_until_online(ip)?;
    println!("Fetching the hash of the image on slot1");
    // run app info and capture stdout
    let out = Command::new(&mcumgr)
        .args(["-t","udp","-d", ip, "app","info"])
        .output()?;
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // extract hash from slot 1
    let mut cur_slot: Option<u8> = None;
    let mut hash_slot1: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("slot:") {
            cur_slot = rest.trim().parse::<u8>().ok();
        } else if let Some(rest) = line.strip_prefix("hash:") {
            if cur_slot == Some(1) {
                hash_slot1 = Some(rest.trim().to_string());
                break;
            }
        }
    }
    let hash = hash_slot1.expect("slot 1 hash not found");
    
    println!("Labeling for testing..");
    // set pending + reset
    Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "app", "test", "--hash", &hash])
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

    // confirm
    let out =Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "app", "confirm", "--hash", &hash])
        .output()?;
        if out.status.success() {println!("--- app info after rollback ---\n{}", String::from_utf8_lossy(&out.stdout));}
    Ok(())
}