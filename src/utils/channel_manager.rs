use std::ops::Deref;

use crate::{serenity::*, utils::config::GUILD_ID, RuscordContext, RuscordResult};

#[repr(transparent)]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelManager(ChannelId);

impl ChannelManager {
	/// Deletes the channel and returns a new, refreshed one, with the same
	/// contents as the previous channel.
	pub async fn refresh_channel(&mut self, ctx: &RuscordContext<'_>) -> RuscordResult<()> {
		let channel = self.id();
		let new_channel = channel.delete(ctx.http()).await?;

		// Safety: All channels created by this library are guild channels
		let guild_channel = unsafe { new_channel.guild().unwrap_unchecked() };

		// Ugly code, but necessary because of the awful builder API
		let channel_builder = CreateChannel::new(guild_channel.name()).kind(guild_channel.kind);
		let channel_builder = if let Some(parent) = guild_channel.parent_id {
			channel_builder.category(parent)
		} else {
			channel_builder
		};
		let channel_builder = if let Some(topic) = guild_channel.topic {
			channel_builder.topic(topic)
		} else {
			channel_builder
		};

		let new_channel = GUILD_ID.create_channel(ctx.http(), channel_builder).await?;
		*self = Self(new_channel.id);
		Ok(())
	}

	/// Finds an existing channel or creates a new one
	pub async fn find_or_create_channel(
		ctx: &Context, name: &str, kind: ChannelType, parent_id: impl Into<Option<ChannelId>>,
		description: impl Into<Option<&str>>,
	) -> RuscordResult<Self> {
		let description: Option<&str> = description.into();
		let parent_id = parent_id.into();

		let description = description.map(|desc| {
			if desc.len() >= 1024 {
				warn!("Description is too long, truncating: {}", desc);
				desc.get(..1023).unwrap_or(desc)
			} else {
				desc
			}
		});

		// Attempt to find an existing channel
		if let Some(channel) = GUILD_ID
			.channels(ctx.http.clone())
			.await?
			.values()
			.find(|g| g.name() == name && g.parent_id == parent_id)
		{
			return Ok(Self(channel.id));
		}

		let base_builder = CreateChannel::new(name).kind(kind);
		let builder = if let Some(topic) = description {
			base_builder.topic(topic)
		} else {
			base_builder
		};

		let builder = if let Some(parent_id) = parent_id {
			builder.category(parent_id)
		} else {
			builder
		};

		// Create new channel if not found
		let new_channel = GUILD_ID.create_channel(ctx.http.clone(), builder).await?;
		Ok(Self(new_channel.id))
	}

	pub fn id(&self) -> ChannelId { self.0 }
}

impl Deref for ChannelManager {
	type Target = ChannelId;

	fn deref(&self) -> &Self::Target { &self.0 }
}
