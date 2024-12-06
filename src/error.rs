use thiserror::Error;

#[derive(Error, Debug)]
/// Main error type for this crate.
pub enum Error {
    #[error("Poise error: {0}")]
    Poise(#[from] poise::serenity_prelude::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}
