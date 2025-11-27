// smp-tool/src/ops/shell_grp.rs

use crate::error::Error;
use crate::error::Result;

use reedline::{
    default_emacs_keybindings, DefaultPrompt, DefaultPromptSegment, Emacs, Reedline, Signal,
};
use tracing::debug;

use mcumgr_smp::{
    shell_management::{self, ShellResult},
    smp::SmpFrame,
};

use crate::client::Client;

/// This function sends a shell command to the smp server and expects a response within the timeout
pub async fn transceive(transport: &mut Client, cmd: Vec<String>, sequence: u8) -> Result<String> {
    let ret: SmpFrame<ShellResult> = transport
        .transceive_cbor(&shell_management::shell_command(sequence, cmd))
        .await?;
    debug!("{:?}", ret);

    match ret.data {
        ShellResult::Ok { o, ret: 0 } => Ok(o),
        ShellResult::Ok { o, ret } => Err(Error::TransceiveReturnErrorCode {
            err_code: ret,
            output: o,
        }),
        ShellResult::Err { rc } => Err(Error::ShellResultError(rc)),
    }
}

/// One-shot "exec" command: `smp-tool shell exec <cmd ...>`
pub async fn exec(transport: &mut Client, cmd: Vec<String>, sequence: u8) -> Result<()> {
    let ret: SmpFrame<ShellResult> = transport
        .transceive_cbor(&shell_management::shell_command(sequence, cmd))
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
    Ok(())
}

/// Interactive shell
pub async fn interactive(transport: &mut Client, sequence: u8) -> Result<()> {
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

                let ret: Result<SmpFrame<ShellResult>, _> = transport
                    .transceive_cbor(&shell_management::shell_command(sequence, argv))
                    .await;
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
