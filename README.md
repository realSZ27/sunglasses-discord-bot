## Build
1. Try to build it once to download crates
2. Make sure you have cmake
3. Go to `%userprofile%/registry/src/index.crates.io-1949cf8c6b5b557f/audiopus_sys-0.2.2/opus` and replace `CMakeLists.txt` with the one in this repo. The existing one is made for an ancient version of cmake.
4. Build like any other rust app.

## Usage
[Build](#Build) it first. Then run it with the following environment variables:

| Environment variable    | Description                                                                                  |
|-------------------------|----------------------------------------------------------------------------------------------|
| DISCORD_TOKEN           | Self explanatory. Set to your bot's token                                                    |
| SOTD_CHANNEL_ID         | Song of the day channel id. The id of the channel you want the song of the day to be posted. |
| SONG_REQUEST_CHANNEL_ID | The id of the channel you want the bot to look for song requests in.                         |
| RUST_LOG                | Optional, set to DEBUG or TRACE if you need more verbose logs.                               |







