// smp-tool/src/ops/img_grp.rs

use std::cmp::min;
use std::error::Error;
use std::path::Path;

use std::net::ToSocketAddrs;

use mcumgr_smp::{
    application_management::{self, GetImageStateResult, WriteImageChunkResult},
    smp::SmpFrame,
};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::client::Client;

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// decode "hash" argument from CLI
fn decode_hash_hex(s: &str) -> Result<[u8; 32], String> {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() != 64 {
        return Err(format!("hex hash must be 64 hex chars, got {}", cleaned.len()));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&cleaned[2 * i..2 * i + 2], 16)
            .map_err(|e| e.to_string())?;
    }
    Ok(out)
}


pub fn info(host: impl ToSocketAddrs, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let ret: SmpFrame<GetImageStateResult> =
        transport
            .transceive_cbor(&application_management::get_state(42))?;

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

pub fn flash(
    host: impl ToSocketAddrs, 
    timeout_ms: u64,
    slot: Option<u8>,
    update_file: &Path,
    chunk_size: usize,
    upgrade: bool,
) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let firmware = std::fs::read(update_file)?;

    let mut hasher = Sha256::new();
    hasher.update(&firmware);
    let hash = hasher.finalize();

    println!("Image sha256: {:x}", hash);

    let mut updater = application_management::ImageWriter::new(
        slot,
        firmware.len(),
        Some(&hash),
        upgrade,
    );

    let mut verified = None;
    let mut offset = 0usize;

    while offset < firmware.len() {
        println!("writing {}/{}", offset, firmware.len());
        let chunk = &firmware[offset..min(firmware.len(), offset + chunk_size)];

        let resp_frame: SmpFrame<WriteImageChunkResult> =
            transport.transceive_cbor(&updater.write_chunk(chunk))?;

        match resp_frame.data {
            WriteImageChunkResult::Ok(payload) => {
                offset = payload.off as usize;
                updater.offset = offset;
                verified = payload.match_;
            }
            WriteImageChunkResult::Err(err) => {
                return Err(format!("Err from MCU: {:?}", err).into());
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

    Ok(())
}

pub fn confirm(host: impl ToSocketAddrs, timeout_ms: u64, hash_hex: &str) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let h = decode_hash_hex(hash_hex)?;
    let ret: SmpFrame<GetImageStateResult> =
        transport
            .transceive_cbor(&application_management::set_confirm(h.to_vec(), true, 42))?;
    debug!("{:?}", ret);
    Ok(())
}

pub fn test_next_boot(host: impl ToSocketAddrs, timeout_ms: u64, hash_hex: &str) -> Result<(), Box<dyn Error>> {
    let mut transport: Client = Client::set_transport(host, timeout_ms)
                .map_err(|e| format!("transport error: {e}"))?;
    let h = decode_hash_hex(hash_hex)?;
    let ret: SmpFrame<GetImageStateResult> =
        transport
            .transceive_cbor(&application_management::set_pending(h.to_vec(), true, 42))?;
    debug!("{:?}", ret);
    Ok(())
}
