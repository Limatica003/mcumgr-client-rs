use std::{process::Command, time::{Duration, Instant}};

pub fn wait_until_online(ip: &str) -> anyhow::Result<()> {
    println!("Trying to connect...");
    let mcumgr = assert_cmd::cargo::cargo_bin!("smp-tool");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop { /* Try to get a response from the target in 20s */
        let out = Command::new(&mcumgr)
            .args(["--timeout-ms", "1000", "-t","udp","-d", ip, "app","info"])
            .output()?;
        if out.status.success() { println!("Connected!"); break; }             // target is back
        if Instant::now() >= deadline { panic!("target did not come back"); }
    }
    Ok(())
}