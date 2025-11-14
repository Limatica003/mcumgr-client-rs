use mcumgr_smp::application_management::{self, GetImageStateResult};
use mcumgr_smp::smp::SmpFrame;
use smp_tool::client::Client;
use std::time::{Duration, Instant};

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
