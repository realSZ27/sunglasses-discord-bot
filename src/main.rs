use serenity::all::{Ready, VoiceState, VoiceStateUpdateEvent};
use serenity::prelude::*;
use std::env;
use chrono::Local;
use serenity::async_trait;
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::modules::gust_abdalla::check_voice_channel_occupancy;

mod modules;
use crate::modules::sotd::*;
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("{} is connected!", ready.user.name);

        // run once when the bot starts up
        let should = should_run_sotd(&ctx).await;
        tracing::debug!("Should run SOTD? {}", should);

        if should { post_song_of_the_day(&ctx).await; }

        // schedules the sotd check for every day at noon
        let sched = JobScheduler::new().await.unwrap();

        // "0 * * * * *" "0 12 * * * *"
        sched.add(
            Job::new_async_tz("0 12 * * * *", Local, move |_uuid, _l| {
                let ctx= ctx.clone();
                Box::pin(async move {
                    tracing::info!("running sotd task");
                    let should = should_run_sotd(&ctx).await;
                    tracing::debug!("Should run SOTD? {}", should);

                    if should { post_song_of_the_day(&ctx).await; }
                })
            }).unwrap()
        ).await.unwrap();

        sched.start().await.expect("Couldn't start cron job");

        tracing::info!("started cron job");
    }

    async fn voice_state_update(&self, ctx: Context, _old_voice_state: Option<VoiceState>, voice_state: VoiceState) {
        tracing::debug!("someones voice state updated");
        let _ = check_voice_channel_occupancy(ctx.clone(), voice_state.guild_id.unwrap(), voice_state.channel_id.unwrap());
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    // Create a new instance of the Client, logging in as a bot.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        tracing::error!("Client error: {why:?}");
    }
}