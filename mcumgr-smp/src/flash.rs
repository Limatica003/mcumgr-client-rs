use std::{cmp::min, net::ToSocketAddrs, path::PathBuf, time::Duration};

use sha2::Digest;

use crate::{
    application_management::{ImageWriter, WriteImageChunkResult},
    transport::{smp::CborSmpTransport, udp::UdpTransport},
    SmpFrame,
};

pub fn flash(
    dest_host: impl ToSocketAddrs,
    timeout_ms: u64,
    slot: Option<u8>,
    update_file: &PathBuf,
    chunk_size: usize,
    upgrade: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut transport = {
        let mut t = UdpTransport::new(dest_host)?;
        t.recv_timeout(Some(Duration::from_millis(timeout_ms)))?;
        CborSmpTransport {
            transport: Box::new(t),
        }
    };

    let firmware = std::fs::read(&update_file)?;

    let mut hasher = sha2::Sha256::new();
    hasher.update(&firmware);
    let hash = hasher.finalize();

    let mut updater = ImageWriter::new(slot, firmware.len(), Some(&hash), upgrade);
    let mut verified = None;

    let mut offset = 0;
    while offset < firmware.len() {
        let chunk = &firmware[offset..min(firmware.len(), offset + chunk_size)];

        let resp_frame: SmpFrame<WriteImageChunkResult> =
            transport.transceive_cbor(&updater.write_chunk(chunk), false)?;

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

    if !verified.is_some_and(|verified| verified) {
        Err("image verification failed")?;
    }

    Ok(offset)
}
