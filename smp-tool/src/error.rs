#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    McumgrTransport(#[from] mcumgr_smp::transport::error::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Mismatched length of hex representation: expected {expected}, got: {got}")]
    HashHexLengthMismatch { expected: usize, got: usize },

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("WriteImageChunkError: {0:?}")]
    WriteImageChunkError(mcumgr_smp::application_management::WriteImageChunkError),

    #[error("Transceive got a non zero return code: {err_code} with output {output}")]
    TransceiveReturnErrorCode { err_code: i32, output: String },

    #[error("ShellResult returned error code: {0}")]
    ShellResultError(i32),

    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),

    #[error("Image confirm failed, {0}")]
    Confirm(String),

    #[error(
        "GetImageStateError: {0:?}, please verify that fw is signed with the correct private key!"
    )]
    GetImageStateError(mcumgr_smp::application_management::GetImageStateError),
}

pub type Result<T = (), E = Error> = core::result::Result<T, E>;
