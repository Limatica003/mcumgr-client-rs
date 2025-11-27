// smp-tool/src/ops/img_grp.rs
use crate::error::Result;
use crate::error::Error;
use std::{cmp::min};
use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};

use mcumgr_smp::{
    application_management::{self, GetImageStateResult, WriteImageChunkResult},
    smp::SmpFrame,
};

use tracing::debug;
use crate::client::Client;

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// decode "hash" argument from CLI
fn decode_hash_hex(s: &str) -> Result<[u8; 32]> {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() != 64 {
        return Err(Error::HashHexLengthMismatch { expected: 64, got: cleaned.len() });
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&cleaned[2 * i..2 * i + 2], 16)?;
    }
    Ok(out)
}

pub async fn info(client: &mut Client, sequence: u8) -> Result<()> {
    let ret: SmpFrame<GetImageStateResult> =
        client.transceive_cbor(&application_management::get_state(sequence)).await?;

    match ret.data {
        GetImageStateResult::Ok(payload) => {
            println!("---------------------------------------------------------------------------");
            for img in payload.images {
                if let Some(h) = img.hash {
                    if h.len() == 32 {
                        println!("slot:      {}", img.slot);
                        println!("version:   {}", img.version);
                        println!("active:    {}", img.active);
                        println!("confirmed: {}", img.confirmed);
                        println!("bootable:  {}", img.bootable);
                        println!("pending:   {}", img.pending);
                        println!("hash:      {}", to_hex(&h));
                        println!("---------------------------------------------------------------------------");
                    } else {
                        eprintln!("unexpected hash length: {}", h.len());
                    }
                }
            }
        }
        GetImageStateResult::Err(err) => {
            eprintln!("rc: {}", err.rc);
            if let Some(msg) = err.rsn {
                eprintln!("rsn: {:?}", msg);
            }
        }
    }
    Ok(())
}

pub async fn flash(
    transport: &mut Client,
    slot: Option<u8>,
    update_file: &Path,
    chunk_size: usize,
    upgrade: bool,
    hash: &str
) -> Result<()> {
    let firmware = std::fs::read(update_file)?;

    let decoded = decode_hash_hex(hash)?;
    let hash: &[u8] = &decoded;

    let mut updater = application_management::ImageWriter::new(
        slot,
        firmware.len(),
        Some(hash),
        upgrade,
    );

    let mut verified = None;
    let mut offset = 0usize;

    // progress bar setup
    let total = firmware.len() as u64;
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner} [{bar:40}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );

    while offset < firmware.len() {
        let chunk = &firmware[offset..min(firmware.len(), offset + chunk_size)];

        let resp_frame: SmpFrame<WriteImageChunkResult> =
            transport.transceive_cbor(&updater.write_chunk(chunk)).await?;

        match resp_frame.data {
            WriteImageChunkResult::Ok(payload) => {
                offset = payload.off as usize;
                updater.offset = offset;
                verified = payload.match_;

                // advance progress bar by written chunk size
                pb.set_position(offset as u64);
            }
            WriteImageChunkResult::Err(err) => {
                pb.finish_and_clear();
                return Err(Error::WriteImageChunkError(err));
            }
        }
    }

    pb.finish_with_message("upload complete");

    println!("sent all bytes: {}", offset);

    if let Some(verified) = verified {
        if verified {
            println!("Image verified");
        } else {
            eprintln!("Image verification failed!");
        }
    }

    Ok(())
}

pub async fn confirm(transport: &mut Client, hash_hex: &str, sequence: u8) -> Result<()> {
    let h = decode_hash_hex(hash_hex)?;
    let ret: SmpFrame<GetImageStateResult> =
        transport
            .transceive_cbor(&application_management::set_confirm(h.to_vec(), true, sequence)).await?;
    debug!("{:?}", ret);
    Ok(())
}

pub async fn test_next_boot(transport: &mut Client, hash_hex: &str, sequence: u8) -> Result<()> {
    let h = decode_hash_hex(hash_hex)?;
    let ret: SmpFrame<GetImageStateResult> =
        transport
            .transceive_cbor(&application_management::set_pending(h.to_vec(), true, sequence)).await?;
    debug!("{:?}", ret);
    Ok(())
}
