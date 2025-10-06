use std::collections::HashSet;
use std::env;
use chrono::{Local};
use regex::Regex;
use serenity::all::{ChannelId, Context, GetMessages, Message, MessageId, Http};

pub async fn post_song_of_the_day(ctx: &Context) {
    let http = ctx.as_ref();

    let song_request_channel_id = ChannelId::new(env::var("SONG_REQUEST_CHANNEL_ID")
        .expect("Missing SONG_REQUEST_CHANNEL_ID in env")
        .parse()
        .expect("SONG_REQUEST_CHANNEL_ID must be a u64"));

    let song_of_the_day_channel_id = ChannelId::new(env::var("SOTD_CHANNEL_ID")
        .expect("Missing SOTD_CHANNEL_ID in env")
        .parse()
        .expect("SOTD_CHANNEL_ID must be a u64"));

    let song_request_search = get_all_messages(&http, song_request_channel_id).await.unwrap();
    let sotd_search = get_all_messages(&http, song_of_the_day_channel_id).await.unwrap();

    if let Some(next_song) = find_next_song(&song_request_search, &sotd_search).await {
        tracing::info!("Next song: {} from {}", next_song.content, next_song.author.name);
        song_of_the_day_channel_id.say(&ctx.http, format!("## SONG OF THE DAY {}\n{}", Local::now().format("%b %d, %Y"), next_song.content)).await.expect("TODO: panic message");
    } else {
        tracing::warn!("No new song requests found!");
    }
}

pub async fn should_run_sotd(ctx: &Context) -> bool {
    let song_of_the_day_channel_id = ChannelId::new(env::var("SOTD_CHANNEL_ID")
        .expect("Missing SOTD_CHANNEL_ID in env")
        .parse()
        .expect("SOTD_CHANNEL_ID must be a u64"));

    let builder = GetMessages::new().limit(10);
    let messages = song_of_the_day_channel_id.messages(ctx.http.clone(), builder).await.unwrap();

    let last_msg_opt = messages.into_iter().find(|m| m.thread.is_none());

    if let Some(last_msg) = last_msg_opt {
        tracing::debug!("last top-level message: {}", last_msg.content);
        let last_date = last_msg.timestamp.with_timezone(&Local).date_naive();
        let now = Local::now().date_naive();
        let result = last_date < now; // run if last top-level SOTD was before today
        tracing::debug!("last date: {} now: {}", last_date, now);
        result
    } else {
        tracing::debug!("no top-level messages found yet");
        true // nothing posted yet, run
    }
}

pub async fn get_all_messages(http: &Http, channel_id: ChannelId) -> serenity::Result<Vec<Message>> {
    let mut all_messages = Vec::new();
    let mut last_id: Option<MessageId> = None;

    loop {
        // Build the GetMessages request
        let mut builder = GetMessages::new().limit(100);
        if let Some(id) = last_id {
            builder = builder.before(id);
        }

        // Fetch messages batch
        let batch: Vec<Message> = channel_id.messages(http, builder).await?;

        if batch.is_empty() {
            break;
        }

        all_messages.extend(batch.iter().cloned());

        // Prepare next page: the oldest message in this batch
        last_id = batch.last().map(|m| m.id);
    }

    Ok(all_messages)
}

/// Finds the oldest song request not already in the SOTD channel.
pub async fn find_next_song(
    requests: &[Message],
    sotd_messages: &[Message],
) -> Option<Message> {
    // Regex to extract Spotify links
    let spotify_re = Regex::new(r"https?://open\.spotify\.com/track/[^\s?]+").unwrap();

    // Collect all SOTD links
    let existing_links = collect_links(Vec::from(sotd_messages), &spotify_re);

    // Requests sorted oldest first
    let mut sorted = requests.to_vec();
    sorted.sort_by_key(|msg| msg.id);

    for msg in sorted {
        if let Some(link_match) = spotify_re.find(&msg.content) {
            let link = link_match.as_str();
            if !existing_links.contains(link) {
                return Some(msg);
            }
        }
    }

    None
}
pub async fn print_new_links(ctx: &Context) {
    let http = ctx.as_ref();

    let song_request_channel_id = ChannelId::new(
        env::var("SONG_REQUEST_CHANNEL_ID")
            .expect("Missing SONG_REQUEST_CHANNEL_ID")
            .parse()
            .expect("SONG_REQUEST_CHANNEL_ID must be a u64"),
    );

    let song_of_the_day_channel_id = ChannelId::new(
        env::var("SOTD_CHANNEL_ID")
            .expect("Missing SOTD_CHANNEL_ID")
            .parse()
            .expect("SOTD_CHANNEL_ID must be a u64"),
    );

    // Fetch messages
    let requests = get_all_messages(&http, song_request_channel_id).await.unwrap();
    let sotd_messages = get_all_messages(&http, song_of_the_day_channel_id).await.unwrap();

    // Regex for Spotify links
    let spotify_re = Regex::new(r"https?://open\.spotify\.com/track/[^\s?]+").unwrap();

    // Collect SOTD links
    let existing_links = collect_links(sotd_messages, &spotify_re);

    let all_links = env::var("ALL_LINKS").is_ok();

    let mut count = 0;

    for msg in requests {
        for link_match in spotify_re.find_iter(&msg.content) {
            let link = link_match.as_str();
            if !existing_links.contains(link) {
                count += 1;
                if all_links { tracing::info!("Found new link: {}", link) }
            }
        }
    }

    tracing::info!("There are {} requests not in sotd", count);
}

fn collect_links(sotd_messages: Vec<Message>, spotify_re: &Regex) -> HashSet<String> {
    let existing_links: HashSet<String> = sotd_messages
        .iter()
        .filter_map(|msg| spotify_re.find(&msg.content).map(|m| m.as_str().to_string()))
        .collect();
    existing_links
}