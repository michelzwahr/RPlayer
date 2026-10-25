mod ui;
mod cover;
mod spotify;

use rspotify::{
    prelude::*,
    scopes,
    AuthCodePkceSpotify,
    Credentials,
    OAuth,
    Config,
};

use crate::ui::UI;
use crossterm::event::{Event, EventStream, KeyCode};
use crossterm::style::Color;
use futures_util::StreamExt;

const DEFAULT_ACCENT: Color = Color::Green;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let client_id = std::env::var("SPOTIFY_CLIENT_ID")?;

    let creds = Credentials::new_pkce(&client_id);

    let oauth = OAuth {
        redirect_uri: "http://127.0.0.1:8080/callback".to_string(),
        scopes : scopes!("user-read-playback-state", "user-read-currently-playing", "user-modify-playback-state"),
        ..Default::default()
    };

    let config = Config {
        token_cached: true,
        token_refreshing: true,
        ..Default::default()
    };

    let mut spotify_auth =
        AuthCodePkceSpotify::with_config(creds, oauth, config);

    // Nur beim ersten Start bzw. wenn kein gültiger Cache vorhanden ist:
    let url = spotify_auth.get_authorize_url(None)?;
    spotify_auth.prompt_for_token(&url).await?;

    let spotify = spotify::Spotify{auth: spotify_auth};

    let mut ui = UI::new(None, None, None);

    ui.build()?;

    let mut events = EventStream::new();
    let mut refresh_spotify = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut refresh_ui = tokio::time::interval(std::time::Duration::from_millis(111));

    let mut last_id: String = String::new();
    let mut last_elapsed: u32 = 0;
    let mut total_duration: u64 = 0;
    let mut last_is_playing: bool = false;
    let mut last_name: String = String::new();
    let mut last_artists: Vec<String> = Vec::new();
    let mut last_album: String = String::new();
    let mut last_explicit: bool = false;
    let mut last_url: String = String::new();
    let mut last_shuffle: bool = false;
    let mut last_repeat: u8 = 0;

    macro_rules! draw_last {
        () => {
            ui.draw(
                last_name.clone(),
                last_explicit,
                last_artists.clone(),
                last_album.clone(),
                total_duration,
                last_elapsed as f64 / 1000.0,
                last_is_playing,
                0,
                last_shuffle,
                last_repeat,
            )?;
        };
    }

    loop {
        tokio::select! {
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('q') => break,
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Left => {
                    ui.decrease_width();
                    cover::clear()?;
                    let cover = cover::fetch_cover(&last_url).await?;
                    ui.accent = cover.accent.unwrap_or(DEFAULT_ACCENT);
                    cover::show(&cover.png, (ui.width - 22) as u16, 3, 20, 8)?;
                    draw_last!();
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Right => {
                    ui.increase_width();
                    cover::clear()?;
                    let cover = cover::fetch_cover(&last_url).await?;
                    ui.accent = cover.accent.unwrap_or(DEFAULT_ACCENT);
                    cover::show(&cover.png, (ui.width - 22) as u16, 3, 20, 8)?;
                    draw_last!();
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char(' ') => {
                    if last_is_playing {
                        spotify.pause().await?;
                        last_is_playing = false;
                        draw_last!();
                    } else {
                        spotify.play().await?;
                        last_is_playing = true;
                        draw_last!();
                    }
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('n') => {
                    spotify.next().await?;
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('p') => {
                    spotify.previous().await?;
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('s') => {
                    spotify.shuffle(!last_shuffle).await?;
                    last_shuffle = !last_shuffle;
                    draw_last!();
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('r') => {
                    let new_repeat = match last_repeat {
                        0 => 2,
                        1 => 0,
                        2 => 1,
                        _ => 0,
                    };
                    spotify.repeat(new_repeat).await?;
                    last_repeat = new_repeat;
                    draw_last!();
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('b') => {
                    ui.fluent_bar = !ui.fluent_bar;
                }
                Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('f') => {
                    ui.fit_terminal()?;
                    cover::clear()?;
                    let cover = cover::fetch_cover(&last_url).await?;
                    ui.accent = cover.accent.unwrap_or(DEFAULT_ACCENT);
                    cover::show(&cover.png, (ui.width - 22) as u16, 3, 20, 8)?;
                    draw_last!();
                }
                Some(Err(error)) => return Err(error.into()),
                _ => {}
            },
            _ = refresh_spotify.tick() => {
                match spotify.playback_info().await {
                    
                    Ok(Some(track)) => {
                        let elapsed = track.current_ms;
                        last_is_playing = track.is_playing;
                        total_duration = (track.duration_ms / 1000) as u64;
                        last_elapsed = elapsed;
                        last_name = track.name.clone();
                        last_artists = track.artists.clone();
                        last_album = track.album.clone();
                        last_explicit = track.explicit;
                        last_url = track.image_url.clone();
                        last_shuffle = track.shuffle;
                        last_repeat = track.repeat;
                        
                        ui.draw(track.name, track.explicit, track.artists, track.album, (track.duration_ms/1000) as u64, elapsed as f64 / 1000.0, track.is_playing, 0, track.shuffle, track.repeat)?;
                        
                        if last_id != track.id {
                            cover::clear()?;
                            if !track.image_url.is_empty() {
                                let cover = cover::fetch_cover(&track.image_url).await?;
                                ui.accent = cover.accent.unwrap_or(DEFAULT_ACCENT);
                                cover::show(&cover.png, (ui.width - 22) as u16, 3, 20, 8)?;
                                draw_last!();
                            }
                            last_id = track.id;
                        }
                    }
                    Ok(None) => {
                        ui.draw("Keine Wiedergabe".to_string(), false, Vec::new(), String::new(), 0, 0.0, false, 0, false, 0)?;
                    }
                    Err(error) => {
                        ui.draw(format!("Spotify-Fehler: {error:#?}"), false, Vec::new(), String::new(), 0, 0.0, false, 0, false, 0)?;
                    }
                }
            }
            _ = refresh_ui.tick() => {
                if last_is_playing {
                    last_elapsed += 111;
                    ui.update_progress((last_elapsed as f64 / 1000.0).min(total_duration as f64), total_duration)?;
                }
            }
        }
    }

    ui.stop()?;

    Ok(())
}


