// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.

use std::error::Error;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use tracing::{warn};
use tracing_subscriber::prelude::*;

use smp_tool::{client::Client};

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum Transport {
    Udp,
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

    #[arg(short = 'd', long, required_if_eq("transport", "udp"))]
    dest_host: Option<String>,

    #[arg(short = 'p', long, default_value_t = 1337)]
    udp_port: u16,

    #[arg(long, default_value_t = 5000)]
    timeout_ms: u64,

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
    /// Confirm image
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli: Cli = Cli::parse();

    warn!("{:?}", cli);
    let addr =  (cli.dest_host.expect("dest_host required"), cli.udp_port);
    let mut client = Client::new(addr, cli.timeout_ms)?;
    match cli.command {
        // OS group
        Commands::Os(OsCmd::Echo { msg }) => {
            client.echo( msg)?;
        }
        Commands::Os(OsCmd::Reset {}) => {
            client.reset()?;
        }

        // Shell group
        Commands::Shell(ShellCmd::Exec { cmd }) => {
            client.exec(cmd)?;
        }
        Commands::Shell(ShellCmd::Interactive) => {
            client.interactive()?;
        }

        // Application (image) group
        Commands::App(ApplicationCmd::Flash {
            slot,
            update_file,
            chunk_size,
            upgrade,
        }) => {
            let hash = "1f22547da114895af757c9ddba823a12eb7964bab2946b6534ecaea2f71dca0e";
            client.flash(slot, &update_file, chunk_size, upgrade, hash)?;
        }
        Commands::App(ApplicationCmd::Info) => {
            client.info()?;
        }
        Commands::App(ApplicationCmd::Confirm { hash }) => {
            client.confirm(&hash)?;
        }
        Commands::App(ApplicationCmd::Test { hash }) => {
            client.test_next_boot(&hash)?;
        }
    }

    Ok(())
}
