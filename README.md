# spotify-osc

Rust app to send spotify related data to VRChat via Open Sound Control (OSC), it also works with other software outside VRChat but requires tweaking on the config file.

I created this a while ago to show spotify data on my VRChat avatar, it's full of poor design decisions and may be a bit unstable since I'm kinda new to Rust.

## OSC Addresses

Addresses can be changed on the configuration to work in other programs, for now it's set to VRChat's avatar parameter addresses.

### Send (App to Client)

| Address                            | Datatype          |
|------------------------------------|-------------------|
| /avatar/parameters/spotify_playing | Boolean           |
| /avatar/parameters/spotify_seek    | Float (Range 0-1) |

## Setup

### Initial setup. 

1. Start the application and close it to generate the config file.
2. Visit https://developer.spotify.com/dashboard/applications
3. Create a new app and give it a cool naem like **"My coow spotify appwication owo"**
4. Add a Redirect URI with the value http://localhost:8080/callback (can be changed on the config file)
5. Open the `config.toml` and set the client_id and client_secret, both can be obtained from the application dashboard.
6. Start the application.
7. Visit https://localhost:8080/setup
8. Login to Spotify
9. If it didn't explode then it should start working in a few moments
10. Now everything should be working fine, if the token expires it should refresh automatically without user interaction.