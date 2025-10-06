use std::collections::HashSet;
use std::env;
use chrono::Local;
use regex::Regex;
use serenity::all::{ChannelId, Context, GetMessages, Message, MessageId, Http};

/// Holds all environment and constant configuration.
#[derive(Clone, Debug)]
pub struct Config {
    pub song_request_channel_id: ChannelId,
    pub song_of_the_day_channel_id: ChannelId,
    pub all_links: bool,
    pub min_id: u64,
    pub spotify_regex: Regex,
}

impl Config {
    pub fn new() -> Self {
        Self {
            song_request_channel_id: ChannelId::new(
                env::var("SONG_REQUEST_CHANNEL_ID")
                    .expect("Missing SONG_REQUEST_CHANNEL_ID")
                    .parse()
                    .expect("SONG_REQUEST_CHANNEL_ID must be a u64"),
            ),
            song_of_the_day_channel_id: ChannelId::new(
                env::var("SOTD_CHANNEL_ID")
                    .expect("Missing SOTD_CHANNEL_ID")
                    .parse()
                    .expect("SOTD_CHANNEL_ID must be a u64"),
            ),
            all_links: env::var("ALL_LINKS").is_ok(),
            min_id: 1417932789315014746,
            spotify_regex: Regex::new(r"https?://open\.spotify\.com/track/[^\s?]+").unwrap(),
        }
    }
}

pub async fn post_song_of_the_day(ctx: &Context, config: &Config) {
    let http = ctx.as_ref();

    let song_request_search: Vec<Message> = get_all_messages(&http, config.song_request_channel_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|msg| msg.id.get() >= config.min_id)
        .collect();

    let sotd_search = get_all_messages(&http, config.song_of_the_day_channel_id).await.unwrap();

    if let Some(next_song) = find_next_song(&song_request_search, &sotd_search, &config).await {
        tracing::info!("Next song: {}", next_song);
        config
            .song_of_the_day_channel_id
            .say(
                &ctx.http,
                format!(
                    "## SONG OF THE DAY {}\n{}",
                    Local::now().format("%b %d, %Y"),
                    next_song
                ),
            )
            .await
            .expect("Failed to post Song of the Day");
    } else {
        tracing::warn!("No new song requests found!");
    }
}

pub async fn should_run_sotd(ctx: &Context, config: &Config) -> bool {
    let builder = GetMessages::new().limit(10);
    let messages = config
        .song_of_the_day_channel_id
        .messages(ctx.http.clone(), builder)
        .await
        .unwrap();

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
        let mut builder = GetMessages::new().limit(100);
        if let Some(id) = last_id {
            builder = builder.before(id);
        }

        let batch: Vec<Message> = channel_id.messages(http, builder).await?;

        if batch.is_empty() {
            break;
        }

        all_messages.extend(batch.iter().cloned());
        last_id = batch.last().map(|m| m.id);
    }

    Ok(all_messages)
}

/// Finds the oldest song request not already in the SOTD channel.
pub async fn find_next_song(
    requests: &[Message],
    sotd_messages: &[Message],
    config: &Config,
) -> Option<String> {
    // Collect existing SOTD links
    let existing_links = collect_links(Vec::from(sotd_messages), &config.spotify_regex);

    // Requests sorted oldest first
    let mut sorted = requests.to_vec();
    sorted.sort_by_key(|msg| msg.id);

    for msg in sorted {
        for link_match in config.spotify_regex.find_iter(&msg.content) {
            let link = link_match.as_str().to_string();
            if !existing_links.contains(&link) {
                return Some(link);
            }
        }
    }

    None
}

pub async fn print_new_links(ctx: &Context, config: &Config) {
    let http = ctx.as_ref();

    let requests: Vec<Message> = get_all_messages(&http, config.song_request_channel_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|msg| msg.id.get() >= config.min_id)
        .collect();

    let sotd_messages = get_all_messages(&http, config.song_of_the_day_channel_id)
        .await
        .unwrap();

    let existing_links = collect_links(sotd_messages, &config.spotify_regex);

    let mut count = 0;

    for msg in requests {
        for link_match in config.spotify_regex.find_iter(&msg.content) {
            let link = link_match.as_str();
            if !existing_links.contains(link) {
                count += 1;
                if config.all_links {
                    tracing::info!("Found new link: {}", link)
                }
            }
        }
    }

    tracing::info!("There are {} requests not in sotd", count);
}

fn collect_links(sotd_messages: Vec<Message>, spotify_re: &Regex) -> HashSet<String> {
    sotd_messages
        .iter()
        .flat_map(|msg| spotify_re.find_iter(&msg.content).map(|m| m.as_str().to_string()))
        .collect()
}
