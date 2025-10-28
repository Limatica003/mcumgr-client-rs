// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.

use std::cmp::min;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use mcumgr_smp::{
    application_management::{self, GetImageStateResult, WriteImageChunkResult},
    os_management::{self, EchoResult, ResetResult},
    shell_management::{self, ShellResult},
    smp::SmpFrame,
    transport::{
        ble::BleTransport,
        serial::SerialTransport,
        smp::{CborSmpTransport, CborSmpTransportAsync},
        udp::UdpTransport,
    },
};
use sha2::Digest;
use tracing::{debug, warn};
use tracing_subscriber::prelude::*;

/// interactive shell support
pub mod shell;

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum Transport {
    Serial,
    Udp,
    Ble,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Command-line tool to send and receive SMP messages.",
    before_help = "Copyright (c) 2023 Gessler GmbH.",
    help_template = "{about-with-newline}\nAuthor: {author-with-newline}{before-help}{usage-heading} {usage}\n\n{all-args}"
)]
struct Cli {
    #[arg(short, long, value_enum)]
    transport: Transport,

    #[arg(short, long, required_if_eq("transport", "serial"))]
    serial_device: Option<String>,

    #[arg(short = 'b', long, default_value_t = 115200)]
    serial_baud: u32,

    #[arg(short = 'd', long, required_if_eq("transport", "udp"))]
    dest_host: Option<String>,

    #[arg(short = 'p', long, default_value_t = 1337)]
    udp_port: u16,

    #[arg(long, default_value_t = 5000)]
    timeout_ms: u64,

    #[arg(short, long, required_if_eq("transport", "ble"))]
    name: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Send a command in the os group
    #[command(subcommand)]
    Os(OsCmd),
    /// Send a command in the shell group
    #[command(subcommand)]
    Shell(ShellCmd),
    /// Send a command in the application group
    #[command(subcommand)]
    App(ApplicationCmd),
}

#[derive(Subcommand, Debug)]
enum OsCmd {
    /// Send an SMP Echo request
    Echo { msg: String },
    /// Send an SMP Reset request
    Reset {},
}
#[derive(Subcommand, Debug)]
enum ShellCmd {
    /// Send a shell command via SMP and read the response
    Exec { cmd: Vec<String> },
    /// Start a remote interactive shell using SMP as the backend
    Interactive,
}
#[derive(Subcommand, Debug)]
enum ApplicationCmd {
    /// Request firmware info
    Info,
    // /// Erase a partition
    // Erase {
    //     #[arg(short, long)]
    //     slot: u8,
    // },
    /// Flash a firmware to an image slot
    Flash {
        #[arg()]
        update_file: PathBuf,
        #[arg(short, long)]
        slot: Option<u8>,
        #[arg(short, long, default_value_t = 256)]
        chunk_size: usize,
        /// Only allow newer firmware versions
        #[arg(long)]
        upgrade: bool,
    },
    /// Confirm image permanantly
    Confirm {
        /// 32-byte hash as hex
        #[arg(long, value_name = "HEX64")]
        hash: String,
    },
    /// Test image in the next boot
    Test {
        /// 32-byte hash as hex
        #[arg(long, value_name = "HEX64")]
        hash: String,
    },
}

fn decode_hash_hex(s: &str) -> Result<[u8; 32], String> {
    // keep only hex chars; allow spaces, colons, 0x, etc.
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() != 64 { return Err(format!("hex hash must be 64 hex chars, got {}", cleaned.len())); }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&cleaned[2*i..2*i+2], 16).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn print_app_info( t: &mut UsedTransport) -> Result<(), mcumgr_smp::transport::error::Error> {
    let ret: SmpFrame<GetImageStateResult> =
        t.transceive_cbor(&application_management::get_state(42)).await?;
    match ret.data {
        GetImageStateResult::Ok(payload) => {
            for img in payload.images {
                if let Some(h) = img.hash {
                    if h.len() == 32 {
                        println!("slot:{}", img.slot);
                        println!("active:{}", img.active);
                        println!("confirmed:{}", img.confirmed);
                        println!("bootable:{}", img.bootable);
                        println!("pending:{}", img.pending);
                        println!("version:{}", img.version);
                        println!("hash:{}", to_hex(&h)); // or hex::encode(&h)
                        println!("------------------------------");
                    } else {
                        eprintln!("unexpected hash length: {}", h.len());
                    }
                }
            }
        }
        GetImageStateResult::Err(err) => {
            eprintln!("rc: {}", err.rc);
            if let Some(msg) = err.rsn { eprintln!("rsn: {:?}", msg); }
        }
    }
    Ok(())
}


pub enum UsedTransport {
    SyncTransport(CborSmpTransport),
    AsyncTransport(CborSmpTransportAsync),
}

impl UsedTransport {
    pub async fn transceive_cbor<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &mut self,
        frame: &SmpFrame<Req>,
    ) -> Result<SmpFrame<Resp>, mcumgr_smp::transport::error::Error> {
        match self {
            UsedTransport::SyncTransport(ref mut t) => t.transceive_cbor(frame, false),
            UsedTransport::AsyncTransport(ref mut t) => t.transceive_cbor(frame, false).await,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli: Cli = Cli::parse();

    warn!("{:?}", cli);

    let mut transport = match cli.transport {
        Transport::Serial => {
            let mut t = SerialTransport::new(
                cli.serial_device.expect("serial device required"),
                cli.serial_baud,
            )?;
            t.recv_timeout(Some(Duration::from_millis(cli.timeout_ms)))?;
            UsedTransport::SyncTransport(CborSmpTransport {
                transport: Box::new(t),
            })
        }
        Transport::Udp => {
            let host = cli.dest_host.expect("dest_host required");
            let port = cli.udp_port;
            debug!("connecting to {} at port {}", host, port);
            let mut t: UdpTransport = UdpTransport::new(
                (host, port),
            )?;
            t.recv_timeout(Some(Duration::from_millis(cli.timeout_ms)))?;
            UsedTransport::SyncTransport(CborSmpTransport {
                transport: Box::new(t),
            })   
        }
        Transport::Ble => {
            let adapters = BleTransport::adapters().await?;
            debug!("found {} adapter(s): {:?}:", adapters.len(), adapters);
            let adapter = adapters.first().ok_or("BLE adapters not found")?;
            debug!("selecting first adapter: {:?}:", adapter);
            UsedTransport::AsyncTransport(CborSmpTransportAsync {
                transport: Box::new(
                    BleTransport::new(
                        cli.name.unwrap(),
                        adapter,
                        Duration::from_millis(cli.timeout_ms),
                    )
                    .await?,
                ),
            })
        }
    };

    match cli.command {
        Commands::Os(OsCmd::Echo { msg }) => {
            let ret: SmpFrame<EchoResult> = transport
                .transceive_cbor(&os_management::echo(42, msg))
                .await?;
            debug!("{:?}", ret);

            match ret.data {
                EchoResult::Ok { r } => {
                    println!("{}", r);
                }
                EchoResult::Err { rc } => {
                    eprintln!("rc: {}", rc);
                }
            }
        }
        Commands::Os(OsCmd::Reset {}) => {
            let ret: SmpFrame<ResetResult> = transport
                .transceive_cbor(&os_management::reset(42, false))
                .await?;
            debug!("{:?}", ret);

            match ret.data {
                ResetResult::Ok { } => {
                    println!("Rebooted");
                }
                ResetResult::Err { rc } => {
                    eprintln!("rc: {}", rc);
                }
            }
        }
        Commands::Shell(ShellCmd::Exec { cmd }) => {
            let ret: SmpFrame<ShellResult> = transport
                .transceive_cbor(&shell_management::shell_command(42, cmd))
                .await?;
            debug!("{:?}", ret);

            match ret.data {
                ShellResult::Ok { o, ret } => {
                    println!("ret: {}, o: {}", ret, o);
                }
                ShellResult::Err { rc } => {
                    eprintln!("rc: {}", rc);
                }
            }
        }
        Commands::Shell(ShellCmd::Interactive) => {
            shell::shell(&mut transport).await?;
        }
        Commands::App(ApplicationCmd::Flash {
            slot,
            update_file,
            chunk_size,
            upgrade,
        }) => {
            let firmware = std::fs::read(&update_file)?;

            let mut hasher = sha2::Sha256::new();
            hasher.update(&firmware);
            let hash = hasher.finalize();

            println!("Image sha256: {:x}", hash);

            let mut updater = mcumgr_smp::application_management::ImageWriter::new(
                slot,
                firmware.len(),
                Some(&hash),
                upgrade,
            );

            let mut verified = None;

            let mut offset = 0;
            while offset < firmware.len() {
                println!("writing {}/{}", offset, firmware.len());
                let chunk = &firmware[offset..min(firmware.len(), offset + chunk_size)];

                let resp_frame: SmpFrame<WriteImageChunkResult> = transport
                    .transceive_cbor(&updater.write_chunk(chunk))
                    .await?;

                match resp_frame.data {
                    WriteImageChunkResult::Ok(payload) => {
                        offset = payload.off as usize;
                        updater.offset = offset;
                        verified = payload.match_;
                    }
                    WriteImageChunkResult::Err(err) => {
                        Err(format!("Err from MCU: {:?}", err))?;
                    }
                }
            }

            println!("sent all bytes: {}", offset);

            if let Some(verified) = verified {
                if verified {
                    println!("Image verified");
                } else {
                    eprintln!("Image verification failed!");
                }
            }
        }
        Commands::App(ApplicationCmd::Info) => {
            print_app_info(&mut transport).await?;
        }
        Commands::App(ApplicationCmd::Confirm { hash }) => {
            let h = decode_hash_hex(&hash)?;
            let ret: SmpFrame<GetImageStateResult> =
                transport.transceive_cbor(&application_management::set_confirm(h.to_vec(), true, 42)).await?;
            debug!("{:?}", ret);
            print_app_info(&mut transport).await?;
        }
        Commands::App(ApplicationCmd::Test { hash }) => {
            let h = decode_hash_hex(&hash)?;
            let ret: SmpFrame<GetImageStateResult> =
                transport.transceive_cbor(&application_management::set_pending(h.to_vec(), true, 42)).await?;
            debug!("{:?}", ret);
            print_app_info(&mut transport).await?;
        }
    }
    Ok(())
}
