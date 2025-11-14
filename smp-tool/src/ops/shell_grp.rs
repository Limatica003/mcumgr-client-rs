// smp-tool/src/ops/shell_grp.rs

use std::error::Error;

use std::net::ToSocketAddrs;

use reedline::{
    default_emacs_keybindings, DefaultPrompt, DefaultPromptSegment, Emacs, Reedline, Signal,
};
use tracing::debug;

use mcumgr_smp::{
    shell_management::{self, ShellResult},
    smp::SmpFrame,
};

use crate::client::Client;

/// One-shot "exec" command: `smp-tool shell exec <cmd ...>`
pub fn exec(host: impl ToSocketAddrs, timeout_ms: u64, cmd: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let ret: SmpFrame<ShellResult> =
        transport
            .transceive_cbor(&shell_management::shell_command(42, cmd))?;
    debug!("{:?}", ret);

    match ret.data {
        ShellResult::Ok { o, ret } => {
            println!("ret: {}, o: {}", ret, o);
        }
        ShellResult::Err { rc } => {
            eprintln!("rc: {}", rc);
        }
    }
    Ok(())
}

/// Interactive shell
pub fn interactive(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let keybindings = default_emacs_keybindings();
    let edit_mode = Box::new(Emacs::new(keybindings));

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("SMP Shell: ".to_string()),
        DefaultPromptSegment::Empty,
    );

    let mut line_editor = Reedline::create().with_edit_mode(edit_mode);

    loop {
        let sig = line_editor.read_line(&prompt)?;

        match sig {
            Signal::Success(buffer) => 'succ: {
                let argv: Vec<_> = buffer.split_whitespace().map(|s| s.to_owned()).collect();

                let ret: Result<SmpFrame<ShellResult>, _> =
                    transport
                        .transceive_cbor(&shell_management::shell_command(42, argv));
                debug!("{:?}", ret);

                let data = match ret {
                    Ok(smp_frame) => smp_frame.data,
                    Err(err) => {
                        println!("transport error: {}", err);
                        break 'succ;
                    }
                };

                match data {
                    ShellResult::Ok { o, ret: _ } => {
                        println!("{}", o);
                    }
                    ShellResult::Err { rc } => {
                        eprintln!("SMP Error: rc: {}", rc);
                    }
                }
            }
            Signal::CtrlD | Signal::CtrlC => {
                println!("\nAborted!");
                break Ok(());
            }
        }
    }
}
