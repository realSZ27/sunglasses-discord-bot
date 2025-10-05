use serenity::all::{ChannelId, Context, GuildId};

pub async fn check_voice_channel_occupancy(ctx: Context, guild_id: GuildId, channel_id: ChannelId) {
    tracing::debug!("running check_voice_channel_occupancy");
    let manager = songbird::get(&ctx).await
        .expect("Songbird Voice client placed at initialisation").clone();

    let guild = match guild_id.to_guild_cached(&ctx.cache) {
        Some(guild) => guild,
        None => return,
    };

    // Correctly count users in the specific voice channel
    let user_count = guild.voice_states.values()
        .filter(|state| state.channel_id == Some(channel_id))
        .count();

    if user_count == 1 {
        play_looping_audio(&ctx, guild_id, channel_id).await;
    } else {
        if let Some(handler_lock) = manager.get(guild_id) {
            let handler = handler_lock.lock().await;
            if handler.current_channel() == Some(channel_id.into()) {
                let _ = manager.leave(guild_id).await;
            }
        }
    }
}

async fn play_looping_audio(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) {
    let manager = songbird::get(&ctx).await
        .expect("Songbird Voice client placed at initialisation")
        .clone();

    // Correctly handle the JoinResult
    let handler_lock = match manager.join(guild_id, channel_id).await {
        Ok(call) => call,
        Err(e) => {
            eprintln!("Error joining voice channel: {:?}", e);

            // Check if we need to leave the server due to gateway state inconsistency
            if e.should_leave_server() {
                let _ = manager.leave(guild_id).await;
            }
            return;
        }
    };

    let mut handler = handler_lock.lock().await;

    // Play the audio file with looping
    let audio_source = songbird::input::File::new("path/to/your/audio/file.mp3");

    let input = songbird::input::Input::from(audio_source);
    let track = handler.play_input(input);

    if let Err(e) = track.enable_loop() {
        eprintln!("Failed to enable looping: {:?}", e);
    }

    println!("Now playing looping audio in voice channel");
}