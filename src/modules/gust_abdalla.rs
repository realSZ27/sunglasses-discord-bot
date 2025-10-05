use serenity::all::{ChannelId, Context, GuildId};

pub async fn check_voice_channel_occupancy(ctx: Context, guild_id: GuildId, channel_id: ChannelId) {
    let manager = songbird::get(&ctx).await
        .expect("Songbird Voice client placed at initialisation").clone();

    // Get the current voice channel states
    let guild = match guild_id.to_guild_cached(&ctx.cache) {
        Some(guild) => guild,
        None => return,
    };

    // Count users in the channel (excluding bots if desired)
    let channel = match guild.voice_states.get(&channel_id) {
        Some(states) => states,
        None => return,
    };

    let user_count = channel.iter()
        .filter(|(_, state)| state.channel_id == Some(channel_id))
        .count();

    // If only one user is in the channel and bot isn't there
    if user_count == 1 {
        // Join the channel and play looping audio
        play_looping_audio(ctx, guild_id, channel_id).await;
    }
    // If multiple users are present and bot is in the channel
    else if user_count > 1 {
        if let Some(handler_lock) = manager.get(guild_id) {
            // Check if bot is in this specific channel
            let handler = handler_lock.lock().await;
            if handler.current_channel() == Some(channel_id.into()) {
                // Leave the voice channel
                let _ = manager.leave(guild_id).await;
            }
        }
    }
}

pub async fn play_looping_audio(ctx: Context, guild_id: GuildId, channel_id: ChannelId) {
    let manager = songbird::get(&ctx).await
        .expect("Songbird Voice client placed at initialisation").clone();

    // Join the voice channel
    let (handler_lock, join_result) = manager.join(guild_id, channel_id).await.unwrap();

    if let Err(e) = join_result {
        eprintln!("Error joining voice channel: {:?}", e);
        return;
    }

    // Play the looping audio file
    let mut handler = handler_lock.lock().await;

    // Load audio from a local file - replace with your file path
    let audio_source = match FileSource::new("path/to/your/audio/file.mp3") {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Error loading audio file: {:?}", e);
            return;
        }
    };

    // Create input from the file source
    let input = songbird::input::Input::from(audio_source);

    // Play the audio and enable looping
    let track = handler.play_input(input);
    track.enable_loop().expect("Failed to enable looping");

    println!("Now playing looping audio in voice channel");
}