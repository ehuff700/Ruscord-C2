use crate::*;
mod recon;
mod utils;
#[macro_export]
macro_rules! say {
    ($ctx:expr, $($args:tt)*) => {{
        let message = format!($($args)*);
        if let Err(why) = $ctx.say(message).await {
            error!("Failed to send message: {}", why);
        }
    }};
}

#[macro_export]
macro_rules! reply {
    ($ctx:expr, $($args:tt)*) => {{
        $ctx.reply(format!($($args)*)).await
    }};
}
 
pub const COMMANDS: [fn() -> poise::Command<crate::Data, crate::Error>; 3] =
    [utils::help, utils::clear, recon::config];

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
