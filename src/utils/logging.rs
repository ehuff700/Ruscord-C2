use std::sync::Arc;

use poise::serenity_prelude::{ChannelId, CreateAttachment, CreateMessage, Http};
use tokio::sync::mpsc;
use tracing::{level_filters::LevelFilter, Metadata};
use tracing_subscriber::{filter::Directive, fmt::MakeWriter};

use crate::RuscordResult;
pub const DISCORD_MAX_MESSAGE_LENGTH: usize = 1999;

#[derive(Debug, Clone, Copy)]
pub enum LoggingLevel {
	Trace,
	Debug,
	Info,
	Warn,
	Error,
}

impl LoggingLevel {
	pub fn as_str(&self) -> &str {
		match self {
			LoggingLevel::Trace => "trace",
			LoggingLevel::Debug => "debug",
			LoggingLevel::Info => "info",
			LoggingLevel::Warn => "warn",
			LoggingLevel::Error => "error",
		}
	}

	pub const fn from_static(str: &'static str) -> Self {
		let bytes = str.as_bytes();

		match bytes {
			b"trace" => LoggingLevel::Trace,
			b"debug" => LoggingLevel::Debug,
			b"info" => LoggingLevel::Info,
			b"warn" => LoggingLevel::Warn,
			b"error" => LoggingLevel::Error,
			_ => LoggingLevel::Debug,
		}
	}
}

impl From<LoggingLevel> for Directive {
	fn from(level: LoggingLevel) -> Self {
		let lfilter = match level {
			LoggingLevel::Trace => LevelFilter::TRACE,
			LoggingLevel::Debug => LevelFilter::DEBUG,
			LoggingLevel::Info => LevelFilter::INFO,
			LoggingLevel::Warn => LevelFilter::WARN,
			LoggingLevel::Error => LevelFilter::ERROR,
		};
		lfilter.into()
	}
}

#[derive(Clone)]
pub struct DiscordWriter {
	log_sender: mpsc::Sender<String>,
}

impl DiscordWriter {
	pub fn new(log_sender: mpsc::Sender<String>) -> Self { Self { log_sender } }
}

impl std::io::Write for DiscordWriter {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let msg = String::from_utf8(buf.to_vec()).unwrap();
		let _ = self.log_sender.try_send(msg);
		Ok(buf.len())
	}

	fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

impl<'a> MakeWriter<'a> for DiscordWriter {
	type Writer = DiscordWriter;

	fn make_writer(&'a self) -> Self::Writer { self.clone() }

	fn make_writer_for(&'a self, _: &Metadata<'_>) -> Self::Writer { self.clone() }
}

/// Starts a logger that sends logs to a Discord channel
pub async fn start_discord_logger(
	log_channel_id: ChannelId, http: Arc<Http>, mut log_receiver: mpsc::Receiver<String>,
) -> RuscordResult<()> {
	while let Some(log_message) = log_receiver.recv().await {
		// Ensure that the log doesn't exceed max length
		let msg = if log_message.len() >= DISCORD_MAX_MESSAGE_LENGTH {
			let attachment = CreateAttachment::bytes(log_message.as_bytes(), "log.txt");
			CreateMessage::new().add_file(attachment)
		} else {
			CreateMessage::new().content(log_message)
		};
		let _ = log_channel_id.send_message(&http, msg).await;
	}
	Ok(())
}
