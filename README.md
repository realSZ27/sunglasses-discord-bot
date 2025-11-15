## Build
1. Try to build it once to download crates
2. Make sure you have cmake
3. Go to `%userprofile%\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\audiopus_sys-0.2.2\opus` and replace `CMakeLists.txt` with the one in this repo. The existing one is made for an ancient version of cmake.
   1. Also, make sure you're building on the release profile. Why? No one knows.
   2. If you're building for linux or mac, just make sure you have the c build tools and a developer version of opus installed ([more info](https://github.com/lakelezz/audiopus?tab=readme-ov-file#building)).
4. Build like any other rust app.

## Usage
[Build](#Build) it first. Then run it with the following environment variables:

| Environment variable    | Description                                                                                                                                                              |
|-------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| DISCORD_TOKEN           | Self explanatory. Set to your bot's token                                                                                                                                |
| SOTD_CHANNEL_ID         | Song of the day channel id. The id of the channel you want the song of the day to be posted.                                                                             |
| SONG_REQUEST_CHANNEL_ID | The id of the channel you want the bot to look for song requests in.                                                                                                     |
| ALL_LINKS               | If this variable is present, it will print all of the links that are waiting in the song requests channel.                                                               |
| SFX_FILE_PATH           | The path to the sound effect played when bot joins an empty call. Must be an opus file format.                                                                           |
| RUST_LOG                | Optional, set to DEBUG or TRACE if you need more verbose logs.                                                                                                           |
| TZ                      | **Only in the docker container.** Optional, your [timezone](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List). By default it will use `America/Chicago` |
| DRY_RUN                 | Optional, runs sotd task as normal, except doesn't send the message at the end. Useful for debugging.                                                                    |
| SKIP_RUN_CHECK          | Optional, runs the sotd task even if there was already one posted today. Useful for debugging.                                                                           |

There is also a docker container in this repo. To run it, do a `git clone https://github.com/realSZ27/sunglasses-discord-bot.git` and modify/rename `docker-compose.example.yaml` to `compose.yaml`.