use serenity::all::{ChannelId, Context, GuildId};
use tracing::{debug};

/// Count non-bot users in the channel using cached guild voice states.
/// Kept private per your request.
async fn count_users_in_channel(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> usize {
    let guild = match ctx.cache.guild(guild_id) {
        Some(g) => g,
        None => {
            debug!("Guild not cached, cannot count users");
            return 0;
        }
    };

    let mut count = 0usize;
    for vs in guild.voice_states.values() {
        if vs.channel_id != Some(channel_id) {
            continue;
        }

        // `vs.user_id` is a UserId (not Option) in current serenity versions.
        let is_bot = ctx
            .cache
            .user(vs.user_id)
            .map(|u| u.bot)
            .unwrap_or(false); // assume human if user not cached

        if !is_bot {
            count += 1;
        }
    }

    debug!("Human users in channel {}: {}", channel_id, count);
    count
}

/// Should the bot leave the channel?
/// Public so you can call it from other files.
pub async fn should_leave(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> bool {
    let manager = songbird::get(ctx).await.expect("Songbird not initialized");

    // If we don't have a handler, we aren't connected; nothing to leave.
    let handler_lock = match manager.get(guild_id) {
        Some(h) => h,
        None => {
            debug!("No handler found for guild {}, not leaving", guild_id);
            return false;
        }
    };
    let handler = handler_lock.lock().await;

    // Count humans only.
    let humans = count_users_in_channel(ctx, guild_id, channel_id).await;

    // Leave if we're connected to that channel and either:
    // - humans == 0 (the solo user left) OR
    // - humans >= 2 (another user joined)
    let should = handler.current_channel() == Some(channel_id.into()) && (humans == 0 || humans >= 2);
    debug!("Should leave? {} (humans = {})", should, humans);
    should
}

/// Should the bot join the channel?
/// Allows joining when handler exists but is not connected (current_channel() == None).
pub async fn should_join(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> bool {
    let manager = songbird::get(ctx).await.expect("Songbird not initialized");

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        match handler.current_channel() {
            Some(ch) => {
                // already connected somewhere — don't start another join
                if ch == channel_id.into() {
                    debug!("Already connected to channel {}, not joining", channel_id);
                } else {
                    debug!("Handler connected to different channel {:?}; not joining", ch);
                }
                return false;
            }
            None => {
                // Handler object exists but it's not connected — allow join to proceed.
                debug!("Handler exists for guild but is not connected; allowing join attempt");
            }
        }
    }

    // Join when exactly 1 human is present.
    let humans = count_users_in_channel(ctx, guild_id, channel_id).await;
    let join = humans == 1;
    debug!("Should join? {} (humans = {})", join, humans);
    join
}

/// Join the channel and start looping audio (public for your module)
pub async fn join_and_play(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) {
    debug!("Attempting to join channel {}", channel_id);

    let manager = songbird::get(ctx).await.expect("Songbird not initialized");

    // Clear any previous queue if a handler exists
    if let Some(existing_lock) = manager.get(guild_id) {
        let existing = existing_lock.lock().await;
        existing.queue().stop();
        debug!("Cleared existing handler queue (if any) before join");
    }

    let call = match manager.join(guild_id, channel_id).await {
        Ok(c) => c,
        Err(e) => {
            debug!("Failed to join voice channel: {:?}", e);
            return;
        }
    };

    debug!("Joined channel {} successfully", channel_id);

    let mut handler = call.lock().await;

    let path = std::env::var("SFX_FILE_PATH").expect("Missing SFX_FILE_PATH");
    let source = songbird::input::File::new(path);
    let track = songbird::tracks::Track::from(source).loops(songbird::tracks::LoopState::Infinite);

    handler.queue().stop(); // clear queue before starting
    let _handle = handler.play(track);

    debug!("Now playing looping audio in channel {}", channel_id);
}

/// Leave the current channel (public for your module)
pub async fn leave_channel(ctx: &Context, guild_id: GuildId) {
    debug!("Attempting to leave channel for guild {}", guild_id);

    let manager = songbird::get(ctx).await.expect("Songbird not initialized");

    // Stop any queued tracks if a handler exists
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().stop();
        debug!("Stopped handler queue for guild {}", guild_id);
        // lock drops here
    }

    // Remove the handler entirely
    match manager.remove(guild_id).await {
        Ok(()) => debug!("Removed handler and left channel for guild {}", guild_id),
        Err(e) => debug!("Failed to remove/leave channel for guild {}: {:?}", guild_id, e),
    }
}