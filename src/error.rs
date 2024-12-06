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
	#[error("Failed to capture screen(s)")]
	XCap(
		#[from]
		#[source]
		xcap::XCapError,
	),
	#[error(transparent)]
	Image(#[from] xcap::image::ImageError),
	#[error("A clipboard error occurred: {0}")]
	Clipboard(#[from] Box<dyn std::error::Error + Send + Sync>),
}
