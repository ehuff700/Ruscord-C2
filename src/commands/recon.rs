use commands::command_channel_check;

use crate::*;

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Displays the agent's configuration
pub async fn config(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    let config = ctx.data().config.read().await;
    reply!(ctx, "```{}```", config.to_string())?;
    Ok(())
}
