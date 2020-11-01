use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(not(feature = "no_std"))]
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
