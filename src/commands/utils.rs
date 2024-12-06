use poise::serenity_prelude::futures::StreamExt;

use crate::{say, RuscordContext, RuscordResult};

/// Displays this help message.
#[poise::command(prefix_command, slash_command)]
pub async fn help(
	ctx: RuscordContext<'_>, #[description = "Specific command to show help about"] command: Option<String>,
) -> RuscordResult<()> {
	let config = poise::builtins::HelpConfiguration {
		show_subcommands: true,
		ephemeral: true,
		extra_text_at_bottom: "\
Type !help command for more info on a command.",
		..Default::default()
	};
	poise::builtins::help(ctx, command.as_deref(), config).await?;
	Ok(())
}

/// Clears the current channel of all messages.
#[poise::command(prefix_command, slash_command)]
pub async fn clear(
	ctx: RuscordContext<'_>,
	#[description = "Amount of messages to delete. If left empty, it will recreate the channel."] count: Option<i32>,
) -> RuscordResult<()> {
	let channel_id = ctx.channel_id();
	let messages = channel_id.messages_iter(&ctx).boxed();

	match count {
		Some(c) => {
			let mut stream = messages.take(c as usize);
			let mut counter = 0;
			while let Some(message_result) = stream.next().await {
				match message_result {
					Ok(message) => {
						message.delete(&ctx.http()).await?;
						counter += 1;
					},
					Err(error) => error!("Error retrieving message: {}", error),
				}
			}

			say!(&ctx, "Successfully deleted {} messages", counter);
		},
		None => {
			let data = ctx.data();
			let mut guard = data.config.write().await;

			// Delete the command channel, and refresh our config with the newly created
			// one.
			let manager = guard.get_manager_for_id(channel_id);
			if let Some(manager) = manager {
				manager.refresh_channel(&ctx).await?;
				manager.say(&ctx.http(), "Messages successfully cleared").await?;
			}
		},
	}
	Ok(())
}
