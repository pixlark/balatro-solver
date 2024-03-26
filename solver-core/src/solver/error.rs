use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("a hand can have a maximum of 5 cards")]
    OverfullHand,
}

pub type Result<T> = std::result::Result<T, Error>;
