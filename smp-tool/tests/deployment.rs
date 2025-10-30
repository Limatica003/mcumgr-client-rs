use assert_cmd::{prelude::*};
use predicates::prelude::*;
use std::{env, process::Command};
use std::thread;
use std::time::{Duration};
mod common;

#[test]
#[ignore] // run manually
fn upload() -> anyhow::Result<()> {
    let mcumgr = assert_cmd::cargo::cargo_bin!("smp-tool");
    let ip = "192.168.2.101";
    let bin_path = "../smp-tool/tests/bin/lcna@3.3.5.bin";
    let hash = "1f22547da114895af757c9ddba823a12eb7964bab2946b6534ecaea2f71dca0e";
    common::wait_until_online(ip)?;
    println!("Loading the image into slot1");
    // upload
    Command::new(mcumgr)
        .args(["-t", "udp", "-d", ip, "app", "flash", bin_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("sent all bytes"));
    
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