#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),
    #[error("SMP: {0}")]
    Smp(#[from] crate::smp::SmpError),
}

pub type Result<T = (), E = Error> = core::result::Result<T, E>;
