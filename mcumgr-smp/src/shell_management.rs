// Author: Sascha Zenglein <zenglein@gessler.de>
// Copyright (c) 2023 Gessler GmbH.
use crate::{Group, SmpFrame};

use crate::OpCode::{WriteRequest, WriteResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ShellCommand {
    /// argv containing cmd + arg, arg, ...
    pub argv: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShellResponse {
    /// argv containing cmd + arg, arg, ...
    pub o: String,
    pub ret: i32
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ShellResult {
    Ok { o: String, ret: i32 },
    Err { rc: i32 },
}

impl ShellResult {
    pub fn into_result(self) -> Result<(String, i32), i32> {
        match self {
            ShellResult::Ok { o, ret } => Ok((o, ret)),
            ShellResult::Err { rc } => Err(rc),
        }
    }
}

pub fn shell_command(sequence: u8, command_args: Vec<String>) -> SmpFrame<ShellCommand> {
    let payload: ShellCommand = ShellCommand { argv: command_args };

    SmpFrame::new(WriteRequest, sequence, Group::ShellManagement, 0, payload)
}

pub fn shell_command_response(sequence: u8, command_args: Vec<String>) -> SmpFrame<ShellResponse> {
    let payload: ShellResponse = ShellResponse { ret: 0, o: command_args.join("") };

    SmpFrame::new(WriteResponse, sequence, Group::ShellManagement, 0, payload)
}
