use serenity::all::{Ready, VoiceState};
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::Local;
use serenity::async_trait;
use songbird::SerenityInit;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{ debug, info };
use tracing_subscriber::EnvFilter;
use crate::modules::gust_abdalla::{join_and_play, leave_channel, should_join, should_leave};

mod modules;

use crate::modules::sotd::*;
struct Handler;

struct SchedCreated;
impl TypeMapKey for SchedCreated { type Value = Arc<AtomicBool>; }

struct SotdLock;
impl TypeMapKey for SotdLock { type Value = Arc<Mutex<()>>; }

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        debug!("Current time is {}", Local::now().format("%H:%M:%S"));

        // Ensure TypeMap entries exist (scheduler flag + mutex)
        {
            let mut data = ctx.data.write().await;
            data.entry::<SchedCreated>()
                .or_insert_with(|| Arc::new(AtomicBool::new(false)));
            data.entry::<SotdLock>()
                .or_insert_with(|| Arc::new(Mutex::new(())));
        }

        let config = Config::new();

        // run once when the bot starts up (take the lock to avoid race with scheduled job)
        {
            // Acquire the mutex so initial run can't race with scheduled run
            let sotd_lock = {
                let data = ctx.data.read().await;
                data.get::<SotdLock>().unwrap().clone()
            };
            let _guard = sotd_lock.lock().await;

            print_new_links(&ctx, &config).await; 
            post_song_of_the_day(&ctx, &config).await; 
        }

        // Only create the scheduler once (guard with the atomic flag)
        let already = {
            let data = ctx.data.read().await;
            data.get::<SchedCreated>().unwrap().load(Ordering::SeqCst)
        };

        if already {
            info!("SOTD scheduler already created; skipping creation in ready handler");
        } else {
            // Attempt to set the flag; if another ready beat us, skip creating scheduler
            let did_set = {
                let data = ctx.data.read().await;
                let flag = data.get::<SchedCreated>().unwrap().clone();
                // swap returns previous value. If it was false, we've set it to true.
                !flag.swap(true, Ordering::SeqCst)
            };

            if did_set {
                let sched = JobScheduler::new().await.unwrap();

                // capture a clone of ctx + config + sotd_lock for use inside the job
                let sotd_lock = {
                    let data = ctx.data.read().await;
                    data.get::<SotdLock>().unwrap().clone()
                };

                sched.add(
                    Job::new_async_tz("0 2 * * * *", Local, move |_uuid, _l| {
                        let ctx = ctx.clone();
                        let config = config.clone();
                        let sotd_lock = sotd_lock.clone();
                        Box::pin(async move {
                            info!("running sotd task");

                            // Acquire the mutex so two scheduler callbacks can't both pass should_run_sotd
                            let _guard = sotd_lock.lock().await;

                            print_new_links(&ctx, &config).await;
                            post_song_of_the_day(&ctx, &config).await;
                        })
                    }).unwrap()
                ).await.unwrap();

                sched.start().await.expect("Couldn't start cron job");
                info!("started cron job");
                // You might want to store the scheduler handle in ctx.data if you need to stop it later
            } else {
                info!("Another ready handler created the scheduler first; skipping creation");
            }
        }
    }
    
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        debug!("Voice state update fired: old={:?}, new={:?}", old, new);

        let guild_id = match new.guild_id.or_else(|| old.as_ref().and_then(|o| o.guild_id)) {
            Some(id) => id,
            None => {
                debug!("No guild id in voice_state_update");
                return;
            }
        };

        let old_channel = old.and_then(|o| o.channel_id);
        let new_channel = new.channel_id;

        // Determine whether the event was caused by a bot (we ignore bot-caused events for decisions)
        let event_is_bot = new
            .member
            .as_ref()
            .map(|m| m.user.bot)
            .unwrap_or(false);

        // ------- HANDLE LEAVE (old_channel -> None or changed) -------
        if let Some(prev_cid) = old_channel {
            // Only respond to humans leaving/joining (ignore bot events).
            if !event_is_bot {
                debug!("Detected change from old channel {} -> {:?}", prev_cid, new_channel);

                // If bot should leave, do that and return.
                if should_leave(&ctx, guild_id, prev_cid).await {
                    debug!("Decided to leave channel {} due to occupancy change", prev_cid);
                    leave_channel(&ctx, guild_id).await;
                    return;
                } else {
                    debug!("Not leaving channel {} after change", prev_cid);

                    // If we didn't need to leave, maybe we should *join* because humans dropped to 1.
                    // This handles the case: channel went 2->1 (someone left) and bot is not connected.
                    if should_join(&ctx, guild_id, prev_cid).await {
                        debug!("Re-joining channel {} because occupant count dropped to 1", prev_cid);
                        join_and_play(&ctx, guild_id, prev_cid).await;
                        return;
                    } else {
                        debug!("After change, not joining channel {} (not eligible)", prev_cid);
                    }
                }
            } else {
                debug!("Skipping leave checks for bot-origin event");
            }
        }

        // ------- HANDLE JOIN (new_channel set) -------
        if let Some(cid) = new_channel {
            // Only consider joining/leave when a human triggered the event.
            if !event_is_bot {
                // <-- NEW: Before trying to join, check whether this join event means the bot should leave
                // (someone else entered the channel, making humans >= 2).
                if should_leave(&ctx, guild_id, cid).await {
                    debug!("Decided to leave channel {} due to occupancy change (join event)", cid);
                    leave_channel(&ctx, guild_id).await;
                    return;
                }

                // If not leaving, maybe join (case: human went 0->1, or handler was not connected)
                if should_join(&ctx, guild_id, cid).await {
                    debug!("Decided to join channel {}", cid);
                    join_and_play(&ctx, guild_id, cid).await;
                } else {
                    debug!("Should not join channel {} (not eligible)", cid);
                }
            } else {
                debug!("Skipping join checks for bot-origin event");
            }
        } else {
            debug!("No channel_id in new voice state — nothing to do for join");
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::new("david_discord_bot_rs=debug")).init();
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client =
        Client::builder(&token, intents)
            .register_songbird()
            .event_handler(Handler)
            .await
            .expect("Error creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        tracing::error!("Client error: {why:?}");
    }
}
